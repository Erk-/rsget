use Streamable;
use regex::Regex;
use serde_json;

use stream_lib::stream::Stream;
use stream_lib::stream::StreamType;

use utils::downloaders::DownloadClient;

use utils::error::StreamError;
use utils::error::RsgetError;

use chrono::prelude::*;

use std::fs::File;

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct XingyanRoomInfo {
    rid: String,
    xid: usize,
    name: String,
    notice: Option<String>,
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
struct XingyanAds {
    title: String,
    img: String,
    linkurl: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct XingyanVideoInfo {
    streamurl: String,
    hlsurl: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct XingyanHostInfo {
    rid: String,
    nickName: String,
    avatar: String,
    //gender: String,
    signature: String,
    is_anchor: String,
}

#[allow(dead_code)]
#[allow(non_snake_case)]
#[derive(Clone, Debug, Deserialize)]
struct XingyanInfo {
    roominfo: XingyanRoomInfo,
    videoinfo: XingyanVideoInfo,
    hostinfo: XingyanHostInfo,
    leftads: Vec<XingyanAds>,
}

#[derive(Clone, Debug)]
pub struct Xingyan {
    client: DownloadClient,
    pub url: String,
    pub room_id: String,
    host_info: XingyanInfo,
}


impl Streamable for Xingyan {
    fn new(url: String) -> Result<Box<Xingyan>, StreamError> {
        let dc = DownloadClient::new()?;
        
        let room_id_re = Regex::new(r"/([0-9]+)")?;
        let cap = match room_id_re.captures(&url) {
            Some(capture) => capture,
            None => return Err(StreamError::Rsget(RsgetError::new("No capture found")))
        };
        let site_url = format!("https://xingyan.panda.tv/{}", &cap[1]);
        let site_req = dc.make_request(&site_url, None)?;
        let res: Result<String, StreamError> = dc.download_to_string(site_req);
        match res {
            Ok(some) => {
                let hostinfo_re = Regex::new(r"<script>window.HOSTINFO=(.*);</script>")?;
                let hi_cap = hostinfo_re
                    .captures(&some)
                    .ok_or_else(|| StreamError::Rsget(RsgetError::new("Regex did not find any hostinfo")))?;
                let hi: XingyanInfo = match serde_json::from_str(&hi_cap[1]) {
                    Ok(info) => info,
                    Err(why) => return Err(StreamError::Json(why)),
                };
                let xy = Xingyan {
                    client: dc,
                    url: url.clone(),
                    room_id: String::from(&cap[1]),
                    host_info: hi,
                };
                debug!("{:#?}", &xy);
                Ok(Box::new(xy))
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
        self.host_info.roominfo.playstatus != "0"
    }

    fn get_stream(&self) -> Result<StreamType, StreamError> {
        Ok(StreamType::Chuncked(self.client.rclient.get(
            &self.host_info.videoinfo.streamurl
        ).build()?))
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

    fn download(&self, path: String) -> Result<u64, StreamError> {
        if !self.is_online() {
            Err(StreamError::Rsget(RsgetError::new("Stream offline")))
        } else {
            println!(
                "{} by {} ({})",
                self.get_title().unwrap(),
                self.get_author().unwrap(),
                self.room_id
            );
            let file = File::create(path)?;
            let stream = Stream::new(self.get_stream()?);
            Ok(stream.write_file(&self.client.rclient, file)?)
        }
    }
}
