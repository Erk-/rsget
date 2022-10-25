use crate::{Status, Streamable};
use regex::Regex;
use stream_lib::DownloadStream;

use crate::utils::error::RsgetError;
use crate::utils::error::StreamError;
use crate::utils::error::StreamResult;

use chrono::prelude::*;

use async_trait::async_trait;

const USER_AGENT: &str = "Mozilla/5.0 (X11; FreeBSD amd64; rv:78.0) Gecko/20100101 Firefox/78.0";

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RoomInitHead {
    data: RoomInit,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct RoomInit {
    room_id: u64,
    live_status: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PlayUrlHead {
    data: PlayUrl,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct PlayUrl {
    durl: Vec<Durl>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Durl {
    url: String,
}

#[derive(Debug, Clone)]
pub struct Bilibili {
    client: reqwest::Client,
    room_id: String,
    room_init: RoomInit,
    durl_list: Vec<Durl>,
}

#[async_trait]
impl Streamable for Bilibili {
    async fn new(url: String) -> StreamResult<Box<Bilibili>> {
        let room_id_re = Regex::new(r"^(?:https?://)?(?:www\.)?live\.bilibili\.com/([0-9]+)")?;

        let cap = match room_id_re.captures(&url) {
            Some(capture) => capture,
            None => return Err(StreamError::Rsget(RsgetError::new("No room_id found"))),
        };
        let room_init_url = format!(
            "https://api.live.bilibili.com/room/v1/Room/room_init?id={}",
            &cap[1]
        );

        let room_id = String::from(&cap[1]);

        let client = reqwest::Client::new();

        let room_init = client
            .get(&room_init_url)
            .send()
            .await?
            .json::<RoomInitHead>()
            .await?
            .data;

        if room_init.live_status != 1 {
            return Err(RsgetError::Offline.into());
        }

        let durls = client
            .get("https://api.live.bilibili.com/room/v1/Room/playUrl")
            .query(&[("cid", &cap[1]), ("quality", "0"), ("platform", "web")])
            .header("User-Agent", USER_AGENT)
            .header("Accept", "*/*")
            .header("Accept-Language", "en-US,en;q=0.5")
            .send()
            .await?
            .json::<PlayUrlHead>()
            .await?
            .data
            .durl;

        Ok(Box::new(Bilibili {
            client,
            room_id,
            room_init,
            durl_list: durls,
        }))
    }

    async fn get_title(&self) -> StreamResult<String> {
        Ok("".to_string())
    }

    async fn get_author(&self) -> StreamResult<String> {
        Ok(self.room_id.clone())
    }

    async fn is_online(&self) -> StreamResult<Status> {
        if self.room_init.live_status == 1 {
            Ok(Status::Online)
        } else {
            Ok(Status::Offline)
        }
    }

    async fn get_stream(&self) -> StreamResult<DownloadStream> {
        Ok(stream_lib::download_chunked(
            self.client.clone(),
            self.client
                .get(&self.durl_list[0].url)
                .header("User-Agent", USER_AGENT)
                .build()?,
        ))
    }

    async fn get_ext(&self) -> StreamResult<String> {
        Ok(String::from("flv"))
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
