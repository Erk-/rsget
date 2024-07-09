// https://www.dr.dk/drtv/kanal/

use std::collections::BTreeMap;

use async_trait::async_trait;
use chrono::{DateTime, Datelike, Local, Timelike};
use regex::Regex;
use stream_lib::DownloadStream;

use crate::{
    utils::error::{RsgetError, StreamError, StreamResult},
    Status, Streamable,
};

pub struct Drdk {
    hls_url: String,
    title: String,
}

#[async_trait]
impl Streamable for Drdk {
    async fn new(url: String) -> StreamResult<Box<Self>>
    where
        Self: Sized + Sync,
    {
        let re_drdk = Regex::new(r"^(?:https?://)?(?:www\.)?dr\.dk/drtv/kanal/[a-zA-Z0-9-_]+$")?;
        if !re_drdk.is_match(&url) {
            return Err(StreamError::Rsget(RsgetError::new("unsupported url")));
        }

        let http = reqwest::Client::new();

        let resp = http.get(&url).send().await?;
        let html = resp.text().await?;

        let window_data_re = Regex::new(r"<script>window.__data = (.+)</script>")?;

        let json = &window_data_re
            .captures(&html)
            .ok_or(StreamError::Rsget(RsgetError::new(
                "Could not find window data",
            )))?[1];

        let mut stream = serde_json::Deserializer::from_str(json.trim()).into_iter::<WindowData>();

        let mut window_data = stream
            .next()
            .ok_or(StreamError::Rsget(RsgetError::new("could not find json")))??;
        let (_id, detail) = window_data
            .cache
            .item_detail
            .pop_first()
            .ok_or(StreamError::Rsget(RsgetError::new(
                "Could not item details",
            )))?;
        let hls_url = detail.item.custom_fields.hls_url;
        let title = detail.item.title;

        Ok(Box::new(Drdk { hls_url, title }))
    }
    async fn get_title(&self) -> StreamResult<String> {
        Ok(self.title.clone())
    }
    async fn get_author(&self) -> StreamResult<String> {
        Ok("DR.DK".to_owned())
    }
    async fn is_online(&self) -> StreamResult<Status> {
        Ok(Status::Unknown)
    }
    async fn get_stream(&self) -> StreamResult<DownloadStream> {
        let http = reqwest::Client::new();
        let request = http.get(&self.hls_url).build()?;
        Ok(stream_lib::download_hls_master_first(http, request, None))
    }
    async fn get_ext(&self) -> StreamResult<String> {
        Ok("ts".to_owned())
    }
    async fn get_default_name(&self) -> StreamResult<String> {
        let local: DateTime<Local> = Local::now();
        Ok(format!(
            "DRTV-{:04}-{:02}-{:02}-{:02}-{:02}-{}.{}",
            local.year(),
            local.month(),
            local.day(),
            local.hour(),
            local.minute(),
            &self.title,
            self.get_ext().await?
        ))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct WindowData {
    cache: Cache,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Cache {
    #[serde(rename = "itemDetail")]
    item_detail: BTreeMap<String, Item>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Item {
    item: ItemDetail,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ItemDetail {
    #[serde(rename = "customFields")]
    custom_fields: CustomFields,
    title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CustomFields {
    #[serde(rename = "hlsURL")]
    hls_url: String,
}
