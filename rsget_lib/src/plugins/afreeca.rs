use Streamable;
use reqwest;
use regex::Regex;

use utils::error::StreamError;
use utils::downloaders::ffmpeg_download;
use chrono::prelude::*;

use url::Url;

use tokio_core::reactor::Core;

use std::process::Command;

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AfreecaGetInfo {
    bid: String,
    mode: String,
    player_type: String
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
    RESULT: usize,
    PBNO: String,
    BNO: String, // ! broadcast
    BJID: String,
    BJNICK: String,
    BJGRADE: usize,
    ISFAV: String,
    CATE: String,
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
    STYPE: String,
    STPT: String,
    CDN: String, // ! cdn
    RMD: String, // ! rmd
    ORG: String,
    MDPT: String,
    BTIME: usize,
    PCON: usize,
    PCON_TIME: String,
    FTK: String,
}

#[allow(non_snake_case)]
#[derive(Clone, Debug, Serialize, Deserialize)]
struct AfreecaChannelInfo<T> {
    CHANNEL: T
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct AfreecaStream{
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
}

// Helper functions
fn get_hls_key(c: reqwest::Client, room_id: String, bno: String) -> String {
    let json_url = Url::parse("http://live.afreecatv.com:8057/afreeca/player_live_api.php").unwrap();
    let reqest_data = AfreecaGetHlsKey {
        bid: room_id,
        bno: bno,
        pwd: String::from(""),
        quality: String::from("original"),
        _type: String::from("pwd"),
    };
    let mut resp = c.post(json_url)
        .form(&reqest_data)
        .send().unwrap();
    let jres: Result<AfreecaChannelInfo<AfreecaHlsKey>, reqwest::Error> = resp.json();
    jres.unwrap().CHANNEL.AID
}

impl Streamable for Afreeca {
    fn new(url: String) -> Result<Box<Afreeca>, StreamError> {
        let room_id_re = Regex::new(r"(?:http://[^/]+)?/([a-zA-Z0-9]+)(?:/[0-9]+)?").unwrap();
        let cap = room_id_re.captures(&url).unwrap();
        let json_url = Url::parse("http://live.afreecatv.com:8057/afreeca/player_live_api.php").unwrap();
        info!("id: {}", &cap[1]);
        let reqest_data = AfreecaGetInfo {
            bid: String::from(&cap[1]),
            mode: String::from("landing"),
            player_type: String::from("html5"),
        };
        
        let client = reqwest::Client::new();
        let mut resp = client.post(json_url)
            .form(&reqest_data)
            .send().unwrap();
        debug!("{}", resp.text().unwrap());
        let jres: Result<AfreecaChannelInfo<AfreecaChannelInfoData>, reqwest::Error> = resp.json();
        match jres {
            Ok(jre) => {
                info!("Sucess when deserialising");
                Ok(Box::new(Afreeca {
                    url: String::from(url.as_str()),
                    room_id: String::from(&cap[1]),
                    afreeca_info: jre.clone(),
                    hls_key: get_hls_key(client, String::from(&cap[1]), jre.CHANNEL.BNO),
                }))},
            Err(why) => {
                info!("Error when deserialising");
                Err(StreamError::Reqwest(why))
            }
        }
    }

    fn get_title(&self) -> Option<String> {
        Some(self.afreeca_info.CHANNEL.TITLE.clone())
    }

    fn get_author(&self) -> Option<String> {
        Some(self.afreeca_info.CHANNEL.BJNICK.clone())
    }

    fn is_online(&self) -> bool {
        match self.afreeca_info.CHANNEL.RESULT {
            0 => false,
            1 => true,
            _ => {
                debug!("Result had value: {}", self.afreeca_info.CHANNEL.RESULT);
                true
            },
        }
    }

    fn get_stream(&self) -> String {
        let json_url = Url::parse(
            &format!("{}/broad_stream_assign.html?return_type={}&broad_key={}",
                     self.afreeca_info.CHANNEL.RMD,
                     self.afreeca_info.CHANNEL.CDN,
                     format!("{}-flash-original-hls", self.afreeca_info.CHANNEL.BNO))).unwrap();
        info!("Stream query url: {}", &json_url);
        info!("CDN: {}", &self.afreeca_info.CHANNEL.CDN.clone());

        let client = reqwest::Client::new();
        let mut resp = client.post(json_url).send().unwrap();
        let jres: AfreecaStream = resp.json().unwrap();
        format!("{}?aid={}", jres.view_url, self.hls_key)
    }

    fn get_ext(&self) -> String {
        String::from("mp4")
    }

    fn get_default_name(&self) -> String {
        let local: DateTime<Local> = Local::now();
        format!(
            "{}-{:04}-{:02}-{:02}-{:02}-{:02}-{}-{}.{}",
            self.room_id,
            local.year(),
            local.month(),
            local.day(),
            local.hour(),
            local.minute(),
            self.get_author().unwrap(),
            self.get_title().unwrap(),
            self.get_ext()
        )
    }

    fn download(&self, _core: &mut Core, path: String) -> Option<()> {
        if !self.is_online() {
            None
        } else {
            println!(
                "{} by {} ({})",
                self.get_title().unwrap(),
                self.get_author().unwrap(),
                self.room_id
            );
            ffmpeg_download(self.get_stream(), path.clone())
        }   
    }
}

