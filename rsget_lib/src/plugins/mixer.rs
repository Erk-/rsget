use crate::Streamable;
use regex::Regex;

use chrono::prelude::*;

use stream_lib::StreamType;

use crate::utils::downloaders::DownloadClient;

use crate::utils::error::RsgetError;
use crate::utils::error::StreamError;



#[derive(Debug, Clone, Serialize, Deserialize)]
struct MixerData {
    id: i64,
    name: String,
}

#[derive(Debug, Clone)]
pub struct Mixer {
    client: DownloadClient,
    url: String,
    username: String,
    data: MixerData,
}

impl Streamable for Mixer {
    fn new(url: String) -> Result<Box<Mixer>, StreamError> {
        let dc = DownloadClient::new()?;

        let room_id_re = Regex::new(r"^(?:https?://)?(?:www\.)?mixer\.com/([a-zA-Z0-9_]+)")?;
        let cap = match room_id_re.captures(&url) {
            Some(capture) => capture,
            None => return Err(StreamError::Rsget(RsgetError::new("No capture found"))),
        };
        let site_url = format!("https://mixer.com/api/v1/channels/{}", &cap[1]);
        let site_req = dc.make_request(&site_url, None)?;
        let res: MixerData = dc.download_and_de(site_req)?;
        Ok(Box::new(Mixer {
            client: dc,
            url: url.clone(),
            username: cap[1].to_string(),
            data: res,
        }))
    }

    fn get_title(&self) -> Option<String> {
        Some(self.data.name.clone())
    }

    fn get_author(&self) -> Option<String> {
        Some(self.username.clone())
    }

    fn is_online(&self) -> bool {
        // Unreachable as Mixer::new will fail for offline channels.
        true
    }

    fn get_stream(&self) -> Result<StreamType, StreamError> {
        Ok(StreamType::NamedPlaylist(
            self.client
                .rclient
                .get(&format!(
                    "https://mixer.com/api/v1/channels/{}/manifest.m3u8",
                    &self.data.id))
                .build()?,
            String::from("source"),
        ))
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
