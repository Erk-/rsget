use Streamable;
use reqwest;
use regex::Regex;

use serde_json;

use utils::error::StreamError;
use utils::error::RsgetError;

use HttpsClient;
use utils::downloaders::download_to_file;
use utils::downloaders::download_and_de;
use utils::downloaders::download_to_string;
use utils::downloaders::make_request;
use chrono::prelude::*;

use tokio_core::reactor::Core;

//use std::{thread, time};
use std::str;
//use std::io::Read;

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
    GWPT: String,
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
fn get_hls_key(client: HttpsClient, room_id: String, bno: String) -> Result<String, StreamError> {
    let mut runtime = Runtime::new()?;
    let reqest_data = AfreecaGetHlsKey {
        bid: room_id,
        bno: bno,
        pwd: String::from(""),
        quality: String::from("original"),
        _type: String::from("pwd"),
    };
    let json_url = format!("http://live.afreecatv.com:8057/afreeca/player_live_api.php?{}",
                           serde_urlencoded::to_string(reqest_data));
    let json_req = make_request(json_url, None)?;
    let jres: Result<AfreecaChannelInfo<AfreecaHlsKey>, reqwest::Error> =
        runtime.block_on(download_and_de::<AfreecaChannelInfo<AfreecaHlsKey>>(client, json_req))?;
    jres?.CHANNEL.AID
}

impl Streamable for Afreeca {
    fn new(client: HttpsClient, url: String) -> Result<Box<Afreeca>, StreamError> {
        let mut runtime = Runtime::new()?;

        let client = client.clone();

        type ChannelInfo = AfreecaChannelInfo<AfreecaChannelInfoData>;
        
        let room_id_re = Regex::new(r"(?:http://[^/]+)?/([a-zA-Z0-9]+)(?:/[0-9]+)?").unwrap();
        let cap = room_id_re.captures(&url).unwrap();
        info!("id: {}", &cap[1]);
        let reqest_data = AfreecaGetInfo {
            bid: String::from(&cap[1]),
            mode: String::from("landing"),
            player_type: String::from("html5"),
        };
        let json_url = format!("http://live.afreecatv.com:8057/afreeca/player_live_api.php?{}",
                               serde_urlencoded::to_string(reqest_data));
        let json_req = make_request(json_url, None)?;
        let jres: Result<ChannelInfo>, StreamError> =
            runtime.block_on(download_and_de::<ChannelInfo>(client, json_req))?;
        match jres {
            Ok(jre) => {
                info!("Sucess when deserialising");
                let retval = Afreeca {
                    url: String::from(url.as_str()),
                    room_id: String::from(&cap[1]),
                    afreeca_info: jre.clone(),
                    hls_key: get_hls_key(client, String::from(&cap[1]), jre.CHANNEL.BNO),
                };
                debug!("Afreeca: {:#?}", retval);
                Ok(Box::new(retval))},
            Err(why) => {
                info!("Error when deserialising, {}", why);
                Err(StreamError::Json(why))
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
        //thread::sleep(time::Duration::from_millis(20000));
        format!("{}?aid={}", jres.view_url, self.hls_key)
    }

    fn get_ext(&self) -> String {
        String::from("mkv")
    }

    fn get_default_name(&self) -> String {
        let local: DateTime<Local> = Local::now();
        format!(
            "{}-{:04}-{:02}-{:02}-{:02}-{:02}-{:02}-{}-{}.{}",
            self.room_id,
            local.year(),
            local.month(),
            local.day(),
            local.hour(),
            local.minute(),
            local.second(),
            self.get_author().unwrap(),
            self.get_title().unwrap(),
            self.get_ext()
        )
    }

    fn download(&self, _core: &mut Core, path: String) -> Result<(), StreamError> {
        if !self.is_online() {
            Err(StreamError::Rsget(RsgetError::new("Stream offline")))
        } else {
            println!(
                "{} by {} ({})",
                self.get_title().unwrap(),
                self.get_author().unwrap(),
                self.room_id
            );
            Err(StreamError::Rsget(RsgetError::new("HLS not implemented")))
        }   
    }
}

