use Streamable;
use std::time::{SystemTime, UNIX_EPOCH};
use regex::Regex;
use serde_json;
use serde_json::Value;

use utils::error::StreamError;
use utils::error::RsgetError;
use utils::downloaders::DownloadClient;
use chrono::prelude::*;

use std::fs::File;

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct PandaTvHostLevel {
    val: f64,
    c_lv: usize,
    c_lv_val: usize,
    n_lv: usize,
    n_lv_val: usize,
    plays_day: f64,
    bamboo_user: f64,
    gift_user: f64,
    gift_cnt: f64,
    vip: usize,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct PandaTvHostInfo {
    rid: usize,
    name: String,
    avatar: String,
    bamboos: String,
    level: PandaTvHostLevel,
}


#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct PandaTvStreamAddr {
    HD: String,
    OD: String,
    SD: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct PandaTvVideoInfo {
    stream_addr: PandaTvStreamAddr,
    room_key: String,
    plflag_list: String,
    plflag: String,
    status: String,
    display_type: String,
    vjjad: usize,
}


#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct PandaTvPictures {
    img: String,
    //qrcode: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct PandaTvRoomInfo {
    id: String,
    name: String,
    #[serde(rename = "type")] pub rtype: String,
    bulletin: String,
    details: String,
    person_num: String,
    classification: String,
    banned_reason: String,
    status: String,
    unlock_time: String,
    watermark_switch: String,
    watermark_loc: String,
    cover_status: String,
    cover_timestamp: usize,
    cover_reason: String,
    //mild_remind_status: usize,
    //mild_remind_timestamp: usize,
    //mild_remind_reason: String,
    account_status: String,
    pictures: PandaTvPictures,
    start_time: String,
    end_time: String,
    room_type: String,
    rtype_value: String,
    show_pbarrage: usize,
    person_time: usize,
    #[serde(skip_deserializing)] pk_stat: usize,
    limitage: usize,
    cate: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct PandaTvUserInfo {
    rid: isize,
    sp_identity: String,
    ispay: bool,
    chat_forbid: bool,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct PandaTvChatConfig {
    min_level: usize,
    all_forbid: usize,
    #[serde(skip_deserializing)] speak_interval: usize,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct PandaTvCallbackParam {
    param: String,
    time: usize,
    sign: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct PandaTvData {
    hostinfo: PandaTvHostInfo,
    videoinfo: PandaTvVideoInfo,
    roominfo: PandaTvRoomInfo,
    userinfo: PandaTvUserInfo,
    chatconfig: PandaTvChatConfig,
    callbackParam: PandaTvCallbackParam,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct PandaTvRoom {
    errno: usize,
    errmsg: String,
    data: PandaTvData,
}

#[derive(Clone, Debug)]
pub struct PandaTv {
    pub url: String,
    pub room_id: String,
    panda_tv_room: PandaTvRoom,
    client: DownloadClient,
}

impl Streamable for PandaTv {
    fn new(url: String) -> Result<Box<PandaTv>, StreamError> {
        let dc = DownloadClient::new()?;

        let room_id_re = Regex::new(r"/([0-9]+)")?;
        let cap = room_id_re.captures(&url).ok_or(StreamError::Rsget(RsgetError::new("[Panda] Could not find roomid")))?;
        let start = SystemTime::now();
        let since_the_epoch = start
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");
        let ts = since_the_epoch.as_secs();
        let json_url = format!(
            "http://www.panda.tv/api_room_v2?roomid={}&__plat=pc_web&_={}",
            &cap[1],
            ts
        );
        let json_req = dc.make_request(&json_url, None)?;
        let jres: Result<PandaTvRoom, StreamError> =
            dc.download_and_de::<PandaTvRoom>(json_req);
        match jres {
            Ok(jre) => {
                let pt = PandaTv {
                    url: String::from(url.as_str()),
                    room_id: String::from(&cap[0]),
                    panda_tv_room: jre,
                    client: dc,
                };
                debug!("{:#?}", pt);
                Ok(Box::new(pt))
            },
            Err(why) => {
                Err(why)
            }
        }
    }
    
    fn get_title(&self) -> Option<String> {
        Some(self.panda_tv_room.data.roominfo.name.clone())
    }

    fn get_author(&self) -> Option<String> {
        Some(self.panda_tv_room.data.hostinfo.name.clone())
    }

    fn is_online(&self) -> bool {
        self.panda_tv_room.data.videoinfo.status == "2"
    }

    fn get_stream(&self) -> String {
        let plflag: Vec<&str> = self.panda_tv_room
            .data
            .videoinfo
            .plflag
            .split('_')
            .collect();
        let data2: Value =
            serde_json::de::from_str(&self.panda_tv_room.data.videoinfo.plflag_list).unwrap();
        let rid = &data2["auth"]["rid"].as_str().unwrap();
        let sign = &data2["auth"]["sign"].as_str().unwrap();
        let ts = &data2["auth"]["time"].as_str().unwrap();

        format!(
            "http://pl{}.live.panda.tv/live_panda/{}.flv?sign={}&ts={}&rid={}",
            plflag[1],
            self.panda_tv_room.data.videoinfo.room_key,
            sign,
            ts,
            rid
        )
    }

    fn get_ext(&self) -> String {
        String::from("flv")
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

    fn download(&self, path: String) -> Result<(), StreamError> {
        if !self.is_online() {
            Err(StreamError::Rsget(RsgetError::new("Stream offline")))
        } else {
            println!(
                "{} by {} ({})",
                self.get_title().unwrap(),
                self.get_author().unwrap(),
                self.room_id
            );
            self.client.download_to_file(
                &self.get_stream(),
                File::create(path)?,
                true,
            )
        }
    }
}
