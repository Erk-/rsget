use crate::{Streamable, Status};
use regex::Regex;

use crate::utils::error::RsgetError;
use crate::utils::error::StreamError;

use chrono::prelude::*;

use reqwest::header::REFERER;
use reqwest::Client as RClient;

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
    pwd: String,
    quality: String,
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
    geo_cc: String,
    geo_rc: String,
    acpt_lang: String,
    svc_lang: String,
    RESULT: i64,
    PBNO: String,
    BNO: String,
    BJID: String,
    BJNICK: String,
    BJGRADE: i64,
    ISFAV: String,
    CATE: String,
    ADCATE: String,
    GRADE: String,
    BTYPE: String,
    CHATNO: String,
    BPWD: String,
    TITLE: String,
    BPS: String,
    RESOLUTION: String,
    CTIP: String,
    CTPT: String,
    CHIP: String,
    CHPT: String,
    GWIP: String,
    GWPT: String,
    STYPE: String,
    STPT: String,
    CDN: String,
    RMD: String,
    ORG: String,
    MDPT: String,
    BTIME: i64,
    PCON: i64,
    #[serde(skip_deserializing, skip_serializing)]
    PCON_TIME: String,
    FTK: String,
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
}

// Helper functions
async fn get_hls_key(
    client: reqwest::Client,
    url: String,
    room_id: String,
    bno: String,
) -> Result<String, StreamError> {
    // http://play.afreecatv.com/rrvv17/207524505
    //CHANNEL_API_URL = "http://live.afreecatv.com:8057/afreeca/player_live_api.php"
    let data = AfreecaGetHlsKey {
        bid: room_id,
        bno,
        pwd: "".to_string(),
        quality: "original".to_string(),
        _type: "pwd".to_string(),
    };
    let mut res = client
        .post("http://live.afreecatv.com:8057/afreeca/player_live_api.php")
        .header(REFERER, url)
        .form(&data)
        .send().await?;
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
    async fn new(url: String) -> Result<Box<Afreeca>, StreamError> {
        type ChannelInfo = AfreecaChannelInfo<AfreecaChannelInfoData>;
        let client = reqwest::Client::new();
        let room_id_re = Regex::new(r"(?:http://[^/]+)?/([a-zA-Z0-9]+)(?:/[0-9]+)?")?;
        let cap = room_id_re.captures(&url).unwrap();
        let room_id = String::from(&cap[1]);
        debug!("room_id: {}", room_id);
        let ci = {
            let data = AfreecaGetInfo {
                bid: room_id,
                mode: String::from("landing"),
                player_type: String::from("html5"),
            };
            let mut res = client
                .post("http://live.afreecatv.com:8057/afreeca/player_live_api.php")
                .form(&data)
                .send().await?;
            debug!("Gettin channel_info");
            let json_str = res.text().await?;
            debug!("{}", json_str);
            let json: ChannelInfo = match serde_json::from_str(&json_str) {
                Ok(s) => s,
                Err(e) => {
                    debug!("[Afreeca] Json failed with:\n{}", e);
                    return Err(StreamError::Rsget(RsgetError::Offline));
                }
            };
            json
        };
        debug!("Getting room_id");
        let hls_key = get_hls_key(
            client.clone(),
            url.clone(),
            String::from(&cap[1]),
            ci.CHANNEL.BNO.clone(),
        ).await?;
        let json_url = format!(
            "{}/broad_stream_assign.html?return_type={}&broad_key={}",
            ci.CHANNEL.RMD.clone(),
            ci.CHANNEL.CDN.clone(),
            format!("{}-flash-original-hls", ci.CHANNEL.BNO.clone())
        );
        debug!("Getting stream_info!");
        let stream_info: AfreecaStream = client.get(&json_url).send().await?.json().await?;
        let retval = Afreeca {
            url: String::from(url.as_str()),
            room_id: String::from(&cap[1]),
            afreeca_info: ci,
            hls_key,
            stream_info,
            client: client,
        };
        debug!("{:#?}", retval);
        Ok(Box::new(retval))
    }

    async fn get_title(&self) -> Result<String, StreamError> {
        Ok(self.afreeca_info.CHANNEL.TITLE.clone())
    }

    async fn get_author(&self) -> Result<String, StreamError> {
        Ok(self.afreeca_info.CHANNEL.BJNICK.clone())
    }

    async fn is_online(&self) -> Result<Status, StreamError> {
        match self.afreeca_info.CHANNEL.RESULT {
            0 => Ok(Status::Offline),
            1 => Ok(Status::Online),
            _ => {
                debug!("Result had value: {}", self.afreeca_info.CHANNEL.RESULT);
                Ok(Status::Unknown)
            }
        }
    }

    async fn get_stream(&self) -> Result<StreamType, StreamError> {
        let cdn = self.afreeca_info.CHANNEL.CDN.clone();
        trace!("CDN: {}", &cdn);
        debug!("view_url: {}", self.stream_info.view_url);
        let url = format!("{}?aid={}", self.stream_info.view_url, self.hls_key);
        unimplemented!();
        /*
        Ok(StreamType::HLS(
            self.client
                .get(&url)
                .header(REFERER, self.url.clone())
                .build()?,
        ))
        */
    }

    async fn get_ext(&self) -> Result<String, StreamError> {
        Ok(String::from("mp4"))
    }

    async fn get_default_name(&self) -> Result<String, StreamError> {
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
