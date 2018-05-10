use Streamable;
use reqwest;
use regex::Regex;
use serde_json;

use utils::downloaders::flv_download;
use utils::error::StreamError;
use chrono::prelude::*;

use tokio_core::reactor::Core;

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct Xingyan2RoomInfo {
    rid: String,
    xid: usize,
    name: String,
    xtype: String,
    level: String,
    photo: String,
    picture: String,
    playstatus: String,
    status: String,
    lock_reason: Option<String>,
    personnum: String,
    starttime: String,
    endtime: String,
    label: Vec<String>,
    shareimg: String,
    detail: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct Xingyan2Ads {
    title: String,
    img: String,
    linkurl: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct Xingyan2StreamTrans {
    mid: String,
    small: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct Xingyan2ZL {
    streamurl: String,
    streamtrans: Xingyan2StreamTrans,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct Xingyan2VideoInfo {
    streamurl: String,
    streamtrans: Xingyan2StreamTrans,
    hlsurl: String,
    zl: Vec<Xingyan2ZL>,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct Xingyan2HostInfo {
    rid: String,
    nickName: String,
    avatar: String,
    gender: String,
    signature: String,
    is_anchor: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct Xingyan2Info {
    roominfo: Xingyan2RoomInfo,
    videoinfo: Xingyan2VideoInfo,
    hostinfo: Xingyan2HostInfo,
}

#[derive(Clone, Debug)]
pub struct Xingyan2 {
    pub url: String,
    pub room_id: String,
    host_info: Xingyan2Info,
}


impl Streamable for Xingyan2 {
    fn new(url: String) -> Result<Box<Xingyan2>, StreamError> {
        let room_id_re = Regex::new(r"/([0-9]+)").unwrap();
        let cap = room_id_re.captures(&url).unwrap();
        let site_url = format!("https://xingyan.panda.tv/{}", &cap[1]);
        let mut resp = reqwest::get(&site_url)?;
        match resp.text() {
            Ok(some) => {
                info!("Unwrapped xinhua");
                let hostinfo_re = Regex::new(r"<script>window.HOSTINFO=(.*);</script>").unwrap();
                let hi_cap = hostinfo_re.captures(&some).unwrap();
                let hi: Xingyan2Info = serde_json::from_str(&hi_cap[1])?;
                let tmp = Xingyan2 {
                    url: url.clone(),
                    room_id: String::from(&cap[1]),
                    host_info: hi,
                };
                debug!("Xingyan2: \n{:?}", &tmp);
                Ok(Box::new(tmp))
            }
            Err(why) => {
                Err(StreamError::Reqwest(why))
            }
        }
    }

    fn get_title(&self) -> Option<String> {
        Some(self.host_info.roominfo.name.clone())
    }

    fn get_author(&self) -> Option<String> {
        Some(self.host_info.hostinfo.nickName.clone())
    }

    fn is_online(&self) -> bool {
        true
        //self.host_info.roominfo.playstatus != "0"
    }

    fn get_stream(&self) -> String {
        self.host_info.videoinfo.streamurl.clone()
    }

    fn get_ext(&self) -> String {
        String::from("flv")
    }

    fn get_default_name(&self) -> String {
        let local: DateTime<Local> = Local::now();
        format!(
            "{:04}-{:02}-{:02}-{:02}-{:02}-{}-{}.{}",
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

    fn download(&self, core: &mut Core, path: String) -> Option<()> {
        if !self.is_online() {
            None
        } else {
            println!(
                "{} by {} ({})",
                self.get_title().unwrap(),
                self.get_author().unwrap(),
                self.room_id
            );
            flv_download(core, self.get_stream(), path)
        }
    }
}
