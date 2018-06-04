use Streamable;
use regex::Regex;

use utils::error::StreamError;
use utils::error::RsgetError;
use chrono::prelude::*;

use utils::downloaders::download_to_file;
use utils::downloaders::download_and_de;
use utils::downloaders::make_request;
use HttpsClient;
use tokio::runtime::current_thread::Runtime;

use std::fs::File;

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct InkeUser {
    uid: usize,
    nick: String,
    gender: usize,
    city: String,
    level: usize,
    pic: String,
    isfollow: usize,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct InkeAddr {
    liveid: String,
    stream_addr: String,
    hls_stream_addr: String,
    rtmp_stream_addr: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct InkeLiveInfo {
    slot: usize,
    user: InkeUser,
    online_users: usize,
    name: String,
    city: String,
    pub_stat: usize,
    landscape: usize,
    rotate: usize,
    live_type: String,
    cover_img: String,
    image: String,
    points: usize,
    liveid: usize,
    status: usize,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct InkeData {
    live_info: InkeLiveInfo,
    room_type: String,
    public_live_info: Option<String>,
    live_addr: Vec<InkeAddr>,
    lived_addr: Vec<InkeAddr>,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
pub struct InkeStruct {
    error_code: usize,
    message: String,
    data: InkeData,
}

#[derive(Clone, Debug)]
pub struct Inke {
    pub url: String,
    pub room_id: String,
    pub inke_info: InkeStruct,
}

impl Streamable for Inke {
    fn new(client: HttpsClient, url: String) -> Result<Box<Inke>, StreamError> {
        let mut runtime = Runtime::new()?;
        let re_inke: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?inke\.cn/live\.html\?uid=([0-9]+)").unwrap();
        let cap = re_inke.captures(&url).unwrap();
        let json_url = format!(
            "http://baseapi.busi.inke.cn/live/LiveInfo?uid={}",
            &cap[1]
        );
        let json_req = make_request(&json_url, None)?;
        let jres = runtime.block_on(download_and_de::<InkeStruct>(client, json_req))?;
        match jres {
            Ok(jre) => Ok(Box::new(Inke {
                url: String::from(url.as_str()),
                room_id: String::from(&cap[1]),
                inke_info: jre,
            })),
            Err(why) => {
                Err(why)
            }
        }
    }

    fn get_title(&self) -> Option<String> {
        Some(self.inke_info.data.live_info.name.clone())
    }

    fn get_author(&self) -> Option<String> {
        Some(self.inke_info.data.live_info.user.nick.clone())
    }

    fn is_online(&self) -> bool {
        self.inke_info.error_code == 0
    }

    fn get_stream(&self) -> String {
        self.inke_info.data.live_addr[0].stream_addr.clone()
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

    fn download(&self, client: HttpsClient, path: String) -> Result<(), StreamError> {
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
