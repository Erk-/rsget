use std::env;

use chrono::prelude::*;
use hls_m3u8::MasterPlaylist;
use rand::{rngs::SmallRng, Rng, SeedableRng};
use regex::Regex;

use std::time::{SystemTime, UNIX_EPOCH};

use stream_lib::StreamType;

use crate::utils::error::RsgetError;
use crate::utils::error::StreamError;
use crate::utils::error::StreamResult;
use crate::{Status, Streamable};

use async_trait::async_trait;

const TWITCH_CLIENT_ID: &str = "fmdejdpeuc71dz6i5q24kpz8kiiynv";
// The reason we need to use this is explained here: https://github.com/streamlink/streamlink/issues/2680#issuecomment-557605851
const TWITCH_CLIENT_ID_PRIVATE: &str = "kimne78kx3ncx6brgo4mv6wki5h1ko";

#[derive(Serialize, Deserialize, Debug)]
pub struct StreamPayload {
    pub data: Vec<StreamData>,
    pub pagination: Pagination,
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Pagination {
    pub cursor: Option<String>,
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
    access_token: Option<String>,
}

#[async_trait]
impl Streamable for Twitch {
    async fn new(url: String) -> StreamResult<Box<Twitch>> {
        let client = reqwest::Client::new();
        let username_re = Regex::new(r"^(?:https?://)?(?:www\.)?twitch\.tv/([a-zA-Z0-9]+)")?;
        let cap = username_re.captures(&url).ok_or_else(|| {
            StreamError::Rsget(RsgetError::new("[Twitch] Cannot capture username"))
        })?;

        let client_id = match env::var("RSGET_TWITCH_CLIENT_ID") {
            Ok(val) => val,
            Err(_) => String::from(TWITCH_CLIENT_ID),
        };

        let access_token = match env::var("RSGET_TWITCH_ACCESS_TOKEN") {
            Ok(val) => Some(val),
            Err(_) => None,
        };

        let twitch = Twitch {
            client,
            username: String::from(&cap[1]),
            url: url.clone(),
            client_id,
            access_token,
        };

        Ok(Box::new(twitch))
    }
    async fn get_title(&self) -> StreamResult<String> {
        if let Some(token) = &self.access_token {
            let stream_url = format!(
                "https://api.twitch.tv/helix/streams?user_login={}",
                self.username
            );
            let payload: StreamPayload = self
                .client
                .get(&stream_url)
                .header("Client-ID", &self.client_id)
                .bearer_auth(token)
                .send()
                .await?
                .json()
                .await
                .map_err(|e| {
                    println!("{}", e);
                    e
                })?;

            match payload.data.get(0) {
                Some(data) => Ok(data.title.clone()),
                None => Err(StreamError::Rsget(RsgetError::new(
                    "[Twitch] User is offline",
                ))),
            }
        } else {
            println!("Access token is not set please complete this flow and set the environment variable RSGET_TWITCH_ACCESS_TOKEN with the value of the access_token after the redirect and rerun");
            let oauth_url = format!(
                "https://id.twitch.tv/oauth2/authorize?client_id={}&redirect_uri={}&response_type=token+id_token&scope=openid",
                self.client_id,
                "http://localhost",
            );

            webbrowser::open(&oauth_url).unwrap();

            Err(StreamError::Rsget(RsgetError::new(
                "[Twitch] No access token is set",
            )))
        }
    }
    async fn get_author(&self) -> StreamResult<String> {
        Ok(self.username.clone())
    }
    async fn is_online(&self) -> StreamResult<Status> {
        if self.get_title().await.is_ok() {
            Ok(Status::Online)
        } else {
            Ok(Status::Offline)
        }
    }
    async fn get_stream(&self) -> StreamResult<StreamType> {
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
        let qu_name = playlist.media.get(0).unwrap().name();

        Ok(StreamType::NamedPlaylist(
            self.client.get(&playlist_url).build()?,
            String::from(qu_name),
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
