use Streamable;
use regex::Regex;
use serde_json;

use utils::downloaders::download_to_file;
use utils::downloaders::download_to_string;
use utils::downloaders::make_request;
use HttpsClient;
use tokio::runtime::current_thread::Runtime;

use utils::error::StreamError;
use utils::error::RsgetError;

use chrono::prelude::*;

use std::fs::File;

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
    fn new(client: &HttpsClient, url: String) -> Result<Box<Xingyan2>, StreamError> {
        let mut runtime = Runtime::new()?;

        let room_id_re = Regex::new(r"/([0-9]+)").unwrap();
        let cap = room_id_re.captures(&url).unwrap();
        let site_url = format!("https://xingyan.panda.tv/{}", &cap[1]);
        let site_req = make_request(&site_url, None)?;
        let res: Result<String, StreamError> = runtime.block_on(
            download_to_string(&client, site_req));

        match res {
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
                debug!("Xingyan2: \n{:#?}", &tmp);
                Ok(Box::new(tmp))
            },
            Err(why) => {
                Err(why)
            },
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

    fn download(&self, client: &HttpsClient, path: String) -> Result<(), StreamError> {
        let mut runtime = Runtime::new()?;
        if !self.is_online() {
            Err(StreamError::Rsget(RsgetError::new("Stream offline")))
        } else {
            println!(
                "{} by {} ({})",
                self.get_title().unwrap(),
                self.get_author().unwrap(),
                self.room_id
            );
            runtime.block_on(
                download_to_file(
                    client,
                    make_request(&self.get_stream(), None)?,
                    File::create(path)?,
                    true)
            ).map(|_|())
        }
    }
}
