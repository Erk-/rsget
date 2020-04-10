use std::env;

use chrono::prelude::*;
use hls_m3u8::MasterPlaylist;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use regex::Regex;

use std::time::{SystemTime, UNIX_EPOCH};

use stream_lib::StreamType;

use crate::utils::error::RsgetError;
use crate::utils::error::StreamError;
use crate::{Status, Streamable};

use async_trait::async_trait;

const TWITCH_CLIENT_ID: &str = "fmdejdpeuc71dz6i5q24kpz8kiiynv";
// The reason we need to use this is explained here: https://github.com/streamlink/streamlink/issues/2680#issuecomment-557605851
const TWITCH_CLIENT_ID_PRIVATE: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";

#[derive(Serialize, Deserialize)]
pub struct StreamPayload {
    pub data: Vec<StreamData>,
    pub pagination: Pagination,
}

#[derive(Serialize, Deserialize)]
pub struct StreamData {
    pub id: String,
    pub user_id: String,
    pub user_name: String,
    pub game_id: String,
    #[serde(rename = "type")]
    pub datum_type: String,
    pub title: String,
    pub viewer_count: i64,
    pub started_at: String,
    pub language: String,
    pub thumbnail_url: String,
}

#[derive(Serialize, Deserialize)]
pub struct Pagination {
    pub cursor: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccessToken {
    token: String,
    sig: String,
}

#[derive(Debug, Clone)]
pub struct Twitch {
    client: reqwest::Client,
    username: String,
    url: String,
    client_id: String,
}

#[async_trait]
impl Streamable for Twitch {
    async fn new(url: String) -> Result<Box<Twitch>, StreamError> {
        let client = reqwest::Client::new();
        let username_re = Regex::new(r"^(?:https?://)?(?:www\.)?twitch\.tv/([a-zA-Z0-9]+)")?;
        let cap = username_re.captures(&url).ok_or_else(|| {
            StreamError::Rsget(RsgetError::new("[Twitch] Cannot capture username"))
        })?;

        let client_id = match env::var("TWITCH_TOKEN") {
            Ok(val) => val,
            Err(_) => String::from(TWITCH_CLIENT_ID),
        };

        let twitch = Twitch {
            client,
            username: String::from(&cap[1]),
            url: url.clone(),
            client_id,
        };

        Ok(Box::new(twitch))
    }
    async fn get_title(&self) -> Result<String, StreamError> {
        let stream_url = format!(
            "https://api.twitch.tv/helix/streams?user_login={}",
            self.username
        );
        let payload: StreamPayload = self
            .client
            .get(&stream_url)
            .header("Client-ID", &self.client_id)
            .send()
            .await?
            .json()
            .await?;

        match payload.data.get(0) {
            Some(data) => Ok(data.title.clone()),
            None => Err(StreamError::Rsget(RsgetError::new(
                "[Twitch] User is offline",
            ))),
        }
    }
    async fn get_author(&self) -> Result<String, StreamError> {
        Ok(self.username.clone())
    }
    async fn is_online(&self) -> Result<Status, StreamError> {
        if self.get_title().await.is_ok() {
            Ok(Status::Online)
        } else {
            Ok(Status::Offline)
        }
    }
    async fn get_stream(&self) -> Result<StreamType, StreamError> {
        let auth_endpoint = format!(
            "https://api.twitch.tv/api/channels/{}/access_token?client_id={}",
            self.username, TWITCH_CLIENT_ID_PRIVATE
        );
        let auth_res = self
            .client
            .get(auth_endpoint.as_str())
            .send()
            .await?
            .text()
            .await?;
        let acs: AccessToken = serde_json::from_str(auth_res.as_str())?;

        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();
        let mut rng = SmallRng::seed_from_u64(time);
        let playlist_url = format!("https://usher.ttvnw.net/api/channel/hls/{}.m3u8?player=twitchweb&token={}&sig={}&allow_audio_only=true&allow_source=true&type=any&p={}",
                                    self.username, acs.token, acs.sig, rng.gen_range(1, 999_999));

        let playlist_res = self
            .client
            .get(playlist_url.as_str())
            .send()
            .await?
            .text()
            .await?;
        let playlist = playlist_res.parse::<MasterPlaylist>()?;
        let qu_name = playlist.media_tags().iter().next().unwrap();

        Ok(StreamType::NamedPlaylist(
            self.client.get(&playlist_url).build()?,
            String::from(qu_name.name().trim()),
        ))
    }
    async fn get_ext(&self) -> Result<String, StreamError> {
        Ok(String::from("mp4"))
    }
    async fn get_default_name(&self) -> Result<String, StreamError> {
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
