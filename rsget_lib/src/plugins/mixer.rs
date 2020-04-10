use regex::Regex;

use chrono::prelude::*;

use crate::utils::error::{RsgetError, StreamError, StreamResult};
use crate::{Status, StreamType, Streamable};

use async_trait::async_trait;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MixerData {
    id: i64,
    name: String,
}

#[derive(Debug, Clone)]
pub struct Mixer {
    client: reqwest::Client,
    url: String,
    username: String,
    data: MixerData,
}

#[async_trait]
impl Streamable for Mixer {
    async fn new(url: String) -> StreamResult<Box<Mixer>> {
        let client = reqwest::Client::new();

        let room_id_re = Regex::new(r"^(?:https?://)?(?:www\.)?mixer\.com/([a-zA-Z0-9_]+)")?;
        let cap = room_id_re
            .captures(&url)
            .ok_or_else(|| StreamError::Rsget(RsgetError::new("[Mixer] No capture found")))?;
        let site_url = format!("https://mixer.com/api/v1/channels/{}", &cap[1]);
        let res: MixerData = client.get(&site_url).send().await?.json().await?;
        Ok(Box::new(Mixer {
            client,
            url: url.clone(),
            username: cap[1].to_string(),
            data: res,
        }))
    }

    async fn get_title(&self) -> StreamResult<String> {
        Ok(self.data.name.clone())
    }

    async fn get_author(&self) -> StreamResult<String> {
        Ok(self.username.clone())
    }

    async fn is_online(&self) -> StreamResult<Status> {
        // Unreachable as Mixer::new will fail for offline channels.
        Ok(Status::Online)
    }

    async fn get_stream(&self) -> StreamResult<StreamType> {
        let url = format!(
            "https://mixer.com/api/v1/channels/{}/manifest.m3u8",
            &self.data.id
        );
        Ok(StreamType::NamedPlaylist(
            self.client.get(&url).build()?,
            String::from("source"),
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
