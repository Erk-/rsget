use std::env;

use chrono::prelude::*;
use hls_m3u8::MasterPlaylist;
use rand::{Rng, thread_rng};
use regex::Regex;
use serde_json;
use serde_json::Value;

use stream_lib::StreamType;

use crate::Streamable;
use crate::utils::downloaders::DownloadClient;
use crate::utils::error::RsgetError;
use crate::utils::error::StreamError;

const TWITCH_CLIENT_ID: &'static str = "fmdejdpeuc71dz6i5q24kpz8kiiynv";

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AccessToken {
    token: String,
    sig: String,
}

#[derive(Debug, Clone)]
pub struct Twitch {
    client: DownloadClient,
    username: String,
    url: String,
    client_id: String,
}

impl Streamable for Twitch {
    fn new(url: String) -> Result<Box<Twitch>, StreamError> {
        let dc = DownloadClient::new()?;
        let username_re = Regex::new(r"^(?:https?://)?(?:www\.)?twitch\.tv/([a-zA-Z0-9]+)")?;
        let cap = match username_re.captures(&url) {
            Some(capture) => capture,
            None => return Err(StreamError::Rsget(RsgetError::new("Cannot capture usernane"))),
        };

        let client_id = match env::var("TWITCH_TOKEN") {
            Ok(val) => val,
            Err(_) => String::from(TWITCH_CLIENT_ID),
        };

        Ok(Box::new(Twitch {
            client: dc,
            username: String::from(&cap[1]),
            url: url.clone(),
            client_id,
        }))

    }
    fn get_title(&self) -> Option<String> {
        let stream_url = format!("https://api.twitch.tv/helix/streams?user_login={}", self.username);
        let stream_req = self.client.make_request(stream_url.as_str(), Some(("Client-ID", self.client_id.as_str()))).unwrap();
        let stream_res = self.client.download_to_string(stream_req).unwrap();
        let inter_json: Value = serde_json::from_str(stream_res.as_str()).unwrap();

        if inter_json["data"].as_array().is_none() ||
           inter_json["data"].as_array().unwrap().is_empty()
        {
            return None;
        }
        Some(String::from(inter_json["data"][0]["title"].as_str().unwrap()))
    }
    fn get_author(&self) -> Option<String> { Some(self.username.clone()) }
    fn is_online(&self) -> bool {
        self.get_title().is_some()
    }
    fn get_stream(&self) -> Result<StreamType, StreamError> {
        let auth_endpoint = format!("https://api.twitch.tv/api/channels/{}/access_token?client_id={}", self.username, self.client_id);
        let auth_req = self.client.make_request(auth_endpoint.as_str(), None)?;
        let auth_res = self.client.download_to_string(auth_req)?;
        let acs: AccessToken = serde_json::from_str(auth_res.as_str())?;

        let mut rng = thread_rng();
        let playlist_url = format!("https://usher.ttvnw.net/api/channel/hls/{}.m3u8?player=twitchweb&token={}&sig={}&allow_audio_only=true&allow_source=true&type=any&p={}",
                                   self.username, acs.token, acs.sig, rng.gen_range(1, 999_999));

        let playlist_req = self.client.make_request(playlist_url.as_str(), None)?;
        let playlist_res = self.client.download_to_string(playlist_req)?;
        let playlist = playlist_res.parse::<MasterPlaylist>()?;
        let qu_name = playlist.media_tags().iter().next().unwrap();

        Ok(StreamType::NamedPlaylist(self.client.rclient.get(playlist_url.as_str()).build()?, String::from(qu_name.name().trim())))

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
    fn get_reqwest_client(&self) -> &reqwest::Client {
        &self.client.rclient
    }
}
