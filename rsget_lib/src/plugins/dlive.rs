use crate::Streamable;
use regex::Regex;
use serde_json;

use stream_lib::Stream;
use stream_lib::StreamType;

use crate::utils::downloaders::DownloadClient;

use crate::utils::error::StreamError;
use crate::utils::error::RsgetError;

use chrono::prelude::*;

use serde_json::Value;

use std::fs::File;

#[derive(Debug, Clone)]
pub struct DLive {
    client: DownloadClient,
    url: String,
    apollo_state: Value,
}

impl Streamable for DLive {
    fn new(url: String) -> Result<Box<DLive>, StreamError> {
        let dc = DownloadClient::new()?;
        
        let room_id_re = Regex::new(r"^(?:https?://)?(?:www\.)?dlive\.tv/([a-zA-Z0-9]+)")?;
        let cap = match room_id_re.captures(&url) {
            Some(capture) => capture,
            None => return Err(StreamError::Rsget(RsgetError::new("No capture found")))
        };
        let site_url = format!("https://dlive.tv/{}", &cap[1]);
        let site_req = dc.make_request(&site_url, None)?;
        let res: Result<String, StreamError> = dc.download_to_string(site_req);
        match res {
            Ok(some) => {
                let apollo_state_re = Regex::new(r"__APOLLO_STATE__=(.*);\(function\(\)")?;
                let apollo_state_cap = apollo_state_re
                    .captures(&some)
                    .ok_or_else(|| StreamError::Rsget(RsgetError::new("Regex did not find any hostinfo")))?;
                let apollo_state: Value = match serde_json::from_str(&apollo_state_cap[1]) {
                    Ok(state) => state,
                    Err(why) => return Err(StreamError::Json(why)),
                };
                
                let xy = DLive {
                    client: dc,
                    url: url.clone(),
                    apollo_state: apollo_state["defaultClient"].as_object().unwrap()
                        .into_iter()
                        .find(|e| e.0.starts_with("user:"))
                        .unwrap().1.clone(),
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
        None
    }

    fn get_author(&self) -> Option<String> {
        Some(self.apollo_state["displayname"].as_str()
                                             .unwrap()
                                             .trim_end_matches('"')
                                             .to_string())
    }

    fn is_online(&self) -> bool {
        !self.apollo_state["livestream"].is_null()
    }

    fn get_stream(&self) -> Result<StreamType, StreamError> {
        Ok(StreamType::NamedPlaylist(self.client.rclient.get(
            &format!("https://live.prd.dlive.tv/hls/live/{}.m3u8", 
                &self.apollo_state["username"].as_str().unwrap()
                    .trim_start_matches("%22")
                    .trim_end_matches("%22"))
        ).build()?, String::from("src")))
    }

    fn get_ext(&self) -> String {
        String::from("mp4")
    }

    fn get_default_name(&self) -> String {
        let local: DateTime<Local> = Local::now();
        format!(
            "{}-{:04}-{:02}-{:02}-{:02}-{:02}.{}",
            self.get_author().unwrap(),
            local.year(),
            local.month(),
            local.day(),
            local.hour(),
            local.minute(),
            self.get_ext()
        )
    }

    fn download(&self, path: String) -> Result<u64, StreamError> {
        if !self.is_online() {
            Err(StreamError::Rsget(RsgetError::new("Stream offline")))
        } else {
            println!(
                "{}",
                self.get_author().unwrap(),
            );
            let file = File::create(path)?;
            let stream = Stream::new(self.get_stream()?);
            Ok(stream.write_file(&self.client.rclient, file)?)
        }
    }
}
