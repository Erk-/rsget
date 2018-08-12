use Streamable;
use regex::Regex;

use utils::error::StreamError;
use utils::error::RsgetError;

use HttpsClient;

use utils::downloaders::DownloadClient;
use chrono::prelude::*;

use tokio::runtime::Runtime;

use std::str;
//use std::fs::File;

//use serde_urlencoded;

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
    //pwd: String,
    mode: String,
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
    client: DownloadClient,
}

// Helper functions
fn get_hls_key(client: &DownloadClient, room_id: String, bno: String) -> Result<String, StreamError> {
    let mut runtime = Runtime::new()?;
    let reqest_data = AfreecaGetHlsKey {
        bid: room_id,
        bno,
        //pwd: String::from(""),
        mode: String::from("landing"),
        quality: String::from("original"),
        _type: String::from("common"),
    };
    let json_url = "http://live.afreecatv.com:8057/afreeca/player_live_api.php";
    let json_req = client.make_request("dr.dk", None)?;//_body(&json_url, None, reqest_data)?;
    let jres: Result<AfreecaChannelInfo<AfreecaHlsKey>, StreamError> =
        client.download_and_de::<AfreecaChannelInfo<AfreecaHlsKey>>(json_req);
    Ok(jres?.CHANNEL.AID)
}

impl Streamable for Afreeca {
    fn new(client: &HttpsClient, url: String) -> Result<Box<Afreeca>, StreamError> {
        let dc = DownloadClient::new(client.clone())?;
        type ChannelInfo = AfreecaChannelInfo<AfreecaChannelInfoData>;
        
        let room_id_re = Regex::new(r"(?:http://[^/]+)?/([a-zA-Z0-9]+)(?:/[0-9]+)?").unwrap();
        let cap = room_id_re.captures(&url).unwrap();
        info!("id: {}", &cap[1]);
        //mode=landing&stream%5Ftype=common&bno=204330439&bid=castle0124
        let reqest_data = AfreecaGetInfo {
            bid: String::from(&cap[1]),
            mode: String::from("landing"),
            player_type: String::from("html5"),
        };
        let json_url = String::from("http://live.afreecatv.com:8057/afreeca/player_live_api.php");

        debug!("_Getting url: {}", &json_url);
        // let json_req = dc.make_request_body(&json_url,
        //                                  None,
        //                                  reqest_data
        // )?;
        panic!("Ehh");
        let json_req = dc.make_request("dr.dk", None)?;
        let jres: Result<ChannelInfo, StreamError> =
            dc.download_and_de::<ChannelInfo>(json_req);
        match jres {
            Ok(jre) => {
                info!("Sucess when deserialising");
                let retval = Afreeca {
                    url: String::from(url.as_str()),
                    room_id: String::from(&cap[1]),
                    afreeca_info: jre.clone(),
                    hls_key: get_hls_key(&dc, String::from(&cap[1]), jre.CHANNEL.BNO)?,
                    client: dc,
                };
                debug!("Afreeca: {:#?}", retval);
                Ok(Box::new(retval))},
            Err(why) => {
                info!("Error when deserialising, {}", why);
                Err(why)
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
        let mut runtime = Runtime::new().unwrap();
        let json_url = format!("{}/broad_stream_assign.html?return_type={}&broad_key={}",
                               self.afreeca_info.CHANNEL.RMD,
                               self.afreeca_info.CHANNEL.CDN,
                               format!("{}-flash-original-hls", self.afreeca_info.CHANNEL.BNO));
        let json_req = self.client.make_request(&json_url, None).unwrap();
        info!("Stream query url: {}", &json_url);
        info!("CDN: {}", &self.afreeca_info.CHANNEL.CDN.clone());
        let jres = self.client.download_and_de::<AfreecaStream>(json_req).unwrap();
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

    fn download(&self, _path: String) -> Result<(), StreamError> {
        if !self.is_online() {
            Err(StreamError::Rsget(RsgetError::new("Stream offline")))
        } else {
            println!(
                "{} by {} ({})",
                self.get_title().unwrap(),
                self.get_author().unwrap(),
                self.room_id
            );
            //http://live-hls-local-cf.afreecatv.com/livestream-east-02/1024x576/204330439-common-original-hls_1.TS
            /*
            #EXTM3U
            #EXT-X-MEDIA-SEQUENCE:1
            #EXT-X-VERSION:3
            #EXT-X-ALLOW-CACHE:NO
            #EXT-X-TARGETDURATION:6
            #EXTINF:6,
            1024x576/204330439-common-original-hls_0.TS
            #EXTINF:6,
            1024x576/204330439-common-original-hls_1.TS
             */
            debug!("STREAM: {}", self.get_stream());
            Err(StreamError::Rsget(RsgetError::new("HLS")))
        }   
    }
}

