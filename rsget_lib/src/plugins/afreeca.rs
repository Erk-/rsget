use Streamable;
use regex::Regex;

use utils::error::StreamError;
use utils::error::RsgetError;

use utils::downloaders::DownloadClient;
use chrono::prelude::*;

use std::str;
use std::fs::File;
    
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
    stream_info: AfreecaStream,
    client: DownloadClient,
}

use reqwest::Client as RClient;
use reqwest::header::REFERER;

// Helper functions
fn get_hls_key(dc: DownloadClient, url: String, room_id: String, bno: String) -> Result<String, StreamError> {
    // http://play.afreecatv.com/rrvv17/207524505
    let rc: RClient = dc.rclient;
    //CHANNEL_API_URL = "http://live.afreecatv.com:8057/afreeca/player_live_api.php"
    let data = AfreecaGetHlsKey {
        bid: room_id,
        bno,
        pwd: "".to_string(),
        quality: "original".to_string(),
        _type: "pwd".to_string(),
    };
    let mut res = rc.post("http://live.afreecatv.com:8057/afreeca/player_live_api.php")
        .header(REFERER, url)
        .form(&data)
        .send()?;
    let json: AfreecaChannelInfo<AfreecaHlsKey> = res.json()?;
    if json.CHANNEL.RESULT != 1 {
        return Err(StreamError::Rsget(RsgetError::new("[Afreeca] HLS key sent back error")));
    }
    Ok(json.CHANNEL.AID)
}


impl Streamable for Afreeca {
    fn new(url: String) -> Result<Box<Afreeca>, StreamError> {
        type ChannelInfo = AfreecaChannelInfo<AfreecaChannelInfoData>;
        let dc = DownloadClient::new()?;
        let room_id_re = Regex::new(r"(?:http://[^/]+)?/([a-zA-Z0-9]+)(?:/[0-9]+)?").unwrap();
        let cap = room_id_re.captures(&url).unwrap();
        let room_id = String::from(&cap[1]);
        debug!("room_id: {}", room_id);
        let ci = {
            let data = AfreecaGetInfo {
                bid: room_id,
                mode: String::from("landing"),
                player_type: String::from("html5"),
            };                
            let mut res = dc
                .rclient
                .post("http://live.afreecatv.com:8057/afreeca/player_live_api.php")
                .form(&data)
                .send()?;
            debug!("Gettin channel_info");
            let json_str = res.text()?;
            debug!("{}", json_str);
            use serde_json;
            let json: ChannelInfo = serde_json::from_str(&json_str)?;
            json
        };
        debug!("Getting room_id");
        let hls_key = get_hls_key(dc.clone(),
                                  url.clone(),
                                  String::from(&cap[1]),
                                   ci.CHANNEL.BNO.clone())?;
        let json_url = format!("{}/broad_stream_assign.html?return_type={}&broad_key={}",
                               ci.CHANNEL.RMD.clone(),
                               ci.CHANNEL.CDN.clone(),
                               format!("{}-flash-original-hls",
                                       ci.CHANNEL.BNO.clone()));
        debug!("Getting stream_info!");
        let stream_info: AfreecaStream = dc.rclient.get(&json_url).send()?.json()?;
        let retval = Afreeca {
            url: String::from(url.as_str()),
            room_id: String::from(&cap[1]),
            afreeca_info: ci,
            hls_key,
            stream_info,
            client: dc,
        };
        debug!("{:#?}", retval);
        Ok(Box::new(retval))
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
        let cdn = self.afreeca_info.CHANNEL.CDN.clone();
        trace!("CDN: {}", &cdn);
        debug!("view_url: {}", self.stream_info.view_url);
        format!("{}?aid={}", self.stream_info.view_url, self.hls_key)
    }

    fn get_ext(&self) -> String {
        String::from("mp4")
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

    fn download(&self, path: String) -> Result<(), StreamError> {
        use std::collections::HashMap;
        if !self.is_online() {
            Err(StreamError::Rsget(RsgetError::new("Stream offline")))
        } else {
            println!(
                "{} by {} ({})",
                self.get_title().unwrap(),
                self.get_author().unwrap(),
                self.room_id
            );
            let mut params = HashMap::new();
            params.insert("aid", &self.hls_key);
            let file = File::create(path)?;
            self.client
                .hls_download(Some(&self.url),
                              Some(self.hls_key.clone()),
                              self.get_stream(),
                              &file)
        }   
    }
}
