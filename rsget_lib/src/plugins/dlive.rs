use crate::{Status, Streamable};
use regex::Regex;
use stream_lib::DownloadStream;
use tracing::debug;

use crate::utils::error::RsgetError;
use crate::utils::error::StreamError;
use crate::utils::error::StreamResult;

use chrono::prelude::*;

use async_trait::async_trait;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct DLive {
    client: reqwest::Client,
    url: String,
    apollo_state: Value,
}

#[async_trait]
impl Streamable for DLive {
    async fn new(url: String) -> StreamResult<Box<DLive>> {
        let client = reqwest::Client::new();

        let room_id_re = Regex::new(r"^(?:https?://)?(?:www\.)?dlive\.tv/([a-zA-Z0-9]+)")?;
        let cap = match room_id_re.captures(&url) {
            Some(capture) => capture,
            None => return Err(StreamError::Rsget(RsgetError::new("No capture found"))),
        };
        let site_url = format!("https://dlive.tv/{}", &cap[1]);
        let res = client.get(&site_url).send().await?.text().await?;

        let apollo_state_re = Regex::new(r"__APOLLO_STATE__=(.*);\(function\(\)")?;
        let apollo_state_cap = apollo_state_re.captures(&res).ok_or_else(|| {
            StreamError::Rsget(RsgetError::new("Regex did not find any hostinfo"))
        })?;
        let apollo_state: Value = match serde_json::from_str(&apollo_state_cap[1]) {
            Ok(state) => state,
            Err(why) => return Err(StreamError::Json(why)),
        };

        let aps = apollo_state["defaultClient"]
            .as_object()
            .ok_or(RsgetError::Offline)?
            .into_iter()
            .find(|e| e.0.starts_with("user:"))
            .ok_or(RsgetError::Offline)?
            .1
            .clone();

        let xy = DLive {
            client,
            url: url.clone(),
            apollo_state: aps,
        };
        debug!("{:#?}", &xy);
        Ok(Box::new(xy))
    }

    async fn get_title(&self) -> StreamResult<String> {
        Ok("".to_string())
    }

    async fn get_author(&self) -> StreamResult<String> {
        Ok(self.apollo_state["displayname"]
            .as_str()
            .unwrap()
            .trim_end_matches('"')
            .to_string())
    }

    async fn is_online(&self) -> StreamResult<Status> {
        if !self.apollo_state["livestream"].is_null() {
            Ok(Status::Online)
        } else {
            Ok(Status::Unknown)
        }
    }

    async fn get_stream(&self) -> StreamResult<DownloadStream> {
        let url = format!(
            "https://live.prd.dlive.tv/hls/live/{}.m3u8",
            &self.apollo_state["username"]
                .as_str()
                .unwrap()
                .trim_start_matches("%22")
                .trim_end_matches("%22")
        );
        Ok(stream_lib::download_hls_named(
            self.client.clone(),
            self.client.get(&url).build()?,
            String::from("src"),
            None,
        ))
    }

    async fn get_ext(&self) -> StreamResult<String> {
        Ok(String::from("mp4"))
    }

    async fn get_default_name(&self) -> StreamResult<String> {
        let local: DateTime<Local> = Local::now();
        Ok(format!(
            "{}-{:04}-{:02}-{:02}-{:02}-{:02}.{}",
            self.get_author().await?,
            local.year(),
            local.month(),
            local.day(),
            local.hour(),
            local.minute(),
            self.get_ext().await?
        ))
    }
}
