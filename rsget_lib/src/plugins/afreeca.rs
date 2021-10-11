use crate::{Status, Streamable};
use regex::Regex;

use crate::utils::error::{RsgetError, StreamError, StreamResult};

use chrono::prelude::*;

use reqwest::header::REFERER;

use stream_lib::StreamType;

use async_trait::async_trait;

use std::str;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AfreecaGetInfo {
    bid: String,
    mode: String,
    player_type: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AfreecaGetHlsKey {
    bid: String,
    bno: String,
    from_api: String,
    mode: String,
    player_type: String,
    pwd: String,
    quality: String,
    stream_type: String,
    #[serde(rename = "type")]
    _type: String,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct AfreecaHlsKey {
    RESULT: usize,
    AID: String,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct AfreecaChannelInfoData {
    RESULT: i64,
    BJNICK: String,
    TITLE: String,
    RMD: String,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct AfreecaChannelInfo<T> {
    CHANNEL: T,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AfreecaStream {
    result: String,
    view_url: String,
    stream_status: String,
}

#[derive(Clone, Debug)]
pub struct Afreeca {
    pub url: String,
    pub room_id: String,
    afreeca_info: AfreecaChannelInfo<AfreecaChannelInfoData>,
    hls_key: String,
    stream_info: AfreecaStream,
    client: reqwest::Client,
    bno: String,
}

// Helper functions
async fn get_hls_key(
    client: reqwest::Client,
    url: String,
    room_id: String,
    bno: String,
) -> StreamResult<String> {
    let data = AfreecaGetHlsKey {
        bid: room_id,
        bno,
        from_api: "0".to_string(),
        mode: "landing".to_string(),
        player_type: "html5".to_string(),
        pwd: "".to_string(),
        quality: "original".to_string(),
        stream_type: "common".to_string(),
        _type: "pwd".to_string(),
    };
    let res = client
        .post("http://live.afreecatv.com/afreeca/player_live_api.php")
        .header(REFERER, url)
        .form(&data)
        .send()
        .await?;
    let json: AfreecaChannelInfo<AfreecaHlsKey> = res.json().await?;
    if json.CHANNEL.RESULT != 1 {
        return Err(StreamError::Rsget(RsgetError::new(
            "[Afreeca] HLS key sent back error",
        )));
    }
    Ok(json.CHANNEL.AID)
}

#[async_trait]
impl Streamable for Afreeca {
    async fn new(mut url: String) -> StreamResult<Box<Afreeca>> {
        if !url.ends_with('/') {
            url.push('/');
        }
        type ChannelInfo = AfreecaChannelInfo<AfreecaChannelInfoData>;
        let client = reqwest::Client::new();
        let room_id_re = Regex::new(r"(?:http://[^/]+)?/([a-zA-Z0-9]+)/([0-9]+)?")?;
        let url_clone = url.clone();
        let cap = room_id_re.captures(&url_clone).ok_or_else(|| {
            StreamError::Rsget(RsgetError::new("[Afreeca] Cannot capture room id"))
        })?;
        let room_id = String::from(&cap[1]);
        let bno = match cap.get(2) {
            Some(_) => String::from(&cap[2]),
            None => {
                warn!("Missing dno");
                let text = client.get(&url).send().await?.text().await?;
                let bno_re = Regex::new(r"var nBroadNo = (\d+);")?;
                let cap = bno_re
                    .captures(&text)
                    .ok_or(StreamError::Rsget(RsgetError::Offline))?;
                cap.get(1).ok_or_else(|| {
                    StreamError::Rsget(RsgetError::new("[Afreeca] Cannot capture bno"))
                })?;
                if !url.ends_with('/') {
                    url.push('/');
                }
                url.push_str(&cap[1]);

                return Self::new(url).await;
            }
        };
        debug!("room_id: {}", room_id);

        debug!("Getting hls_key");
        let hls_key = get_hls_key(
            client.clone(),
            url.clone(),
            String::from(&cap[1]),
            bno.clone(),
        )
        .await?;

        let ci = {
            let data = AfreecaGetInfo {
                bid: room_id,
                mode: String::from("landing"),
                player_type: String::from("html5"),
            };
            let res = client
                .post("http://live.afreecatv.com/afreeca/player_live_api.php")
                .form(&data)
                .send()
                .await?;
            debug!("Gettin channel_info");
            let json_str = res.text().await?;
            debug!("{}", json_str);
            let json: ChannelInfo = serde_json::from_str(&json_str).map_err(|e| {
                debug!("[Afreeca] Json failed with:\n{}", e);
                StreamError::Rsget(RsgetError::Offline)
            })?;
            json
        };
        let json_url = format!(
            "{}/broad_stream_assign.html?return_type=gs_cdn_pc_web&broad_key={}",
            ci.CHANNEL.RMD.clone(),
            format!("{}-flash-original-hls", &bno)
        );
        debug!("Getting stream_info!");
        let stream_info: AfreecaStream = client.get(&json_url).send().await?.json().await?;
        let retval = Afreeca {
            url: String::from(url.as_str()),
            room_id: String::from(&cap[1]),
            afreeca_info: ci,
            hls_key,
            stream_info,
            client,
            bno,
        };
        debug!("{:#?}", retval);
        Ok(Box::new(retval))
    }

    async fn get_title(&self) -> StreamResult<String> {
        Ok(self.afreeca_info.CHANNEL.TITLE.clone())
    }

    async fn get_author(&self) -> StreamResult<String> {
        Ok(self.afreeca_info.CHANNEL.BJNICK.clone())
    }

    async fn is_online(&self) -> StreamResult<Status> {
        match self.afreeca_info.CHANNEL.RESULT {
            0 => Ok(Status::Offline),
            1 => Ok(Status::Online),
            _ => {
                debug!("Result had value: {}", self.afreeca_info.CHANNEL.RESULT);
                Ok(Status::Unknown)
            }
        }
    }

    async fn get_stream(&self) -> StreamResult<StreamType> {
        debug!("view_url: {}", self.stream_info.view_url);
        let url = format!("{}?aid={}", self.stream_info.view_url, self.hls_key);

        Ok(StreamType::HLS(
            self.client
                .get(&url)
                .header(REFERER, self.url.clone())
                .build()?,
        ))
    }

    async fn get_ext(&self) -> StreamResult<String> {
        Ok(String::from("mp4"))
    }

    async fn get_default_name(&self) -> StreamResult<String> {
        let local: DateTime<Local> = Local::now();
        Ok(format!(
            "{}-{:04}-{:02}-{:02}-{:02}-{:02}-{:02}-{}-{}.{}",
            self.room_id,
            local.year(),
            local.month(),
            local.day(),
            local.hour(),
            local.minute(),
            local.second(),
            self.get_author().await.unwrap(),
            self.get_title().await.unwrap(),
            self.get_ext().await.unwrap(),
        ))
    }
}
