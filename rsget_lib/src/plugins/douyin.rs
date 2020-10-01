use async_trait::async_trait;
use regex::Regex;

use stream_lib::StreamType;

use crate::utils::error::{RsgetError, StreamError, StreamResult};
use crate::{Status, Streamable};

/*
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct DouyinRoot {
  #[serde(rename = "hasData")]
  pub has_data: i64,
  #[serde(rename = "videoWidth")]
  pub video_width: i64,
  #[serde(rename = "videoHeight")]
  pub video_height: i64,
  #[serde(rename = "playAddr")]
  pub play_addr: String,
  pub cover: String,
}
*/

#[derive(Clone, Debug)]
pub struct Douyin {
    pub url: String,
    pub video_id: String,
    pub douyin_url: String,
    //DouyinRoot,
    pub description: String,
    pub author: String,
    client: reqwest::Client,
}

#[async_trait]
impl Streamable for Douyin {
    async fn new(url: String) -> StreamResult<Box<Douyin>> {
        let client = reqwest::Client::new();
        let res = client.get(&url).send().await?.text().await;
        match res {
            Ok(some) => {
                let url_re: Regex = Regex::new(
                    r"^(?:https?://)?(?:www\.)?iesdouyin\.com/share/video/([a-zA-Z0-9]+)/.*",
                )?;
                let video_re = Regex::new(r#"playAddr:\s*"(.+)""#)?;
                let description_re = Regex::new(r#"<p class="desc">([^<]*)</p>"#)?;
                let author_re = Regex::new(r#"<p class="name nowrap">@([^<]*)</p>"#)?;
                let id_cap = url_re.captures(&url).ok_or_else(|| {
                    StreamError::Rsget(RsgetError::new("Regex did not find any video id"))
                })?;
                let video_cap = video_re.captures(&some).ok_or_else(|| {
                    StreamError::Rsget(RsgetError::new("Regex did not find any hostinfo"))
                })?;
                let description_cap = description_re.captures(&some).ok_or_else(|| {
                    StreamError::Rsget(RsgetError::new("Regex did not find any description"))
                })?;
                let author_cap = author_re.captures(&some).ok_or_else(|| {
                    StreamError::Rsget(RsgetError::new("Regex did not find any author"))
                })?;

                let ret_val = Douyin {
                    client,
                    url: url.clone(),
                    video_id: String::from(&id_cap[1]),
                    douyin_url: String::from(&video_cap[1]),
                    description: String::from(&description_cap[1]),
                    author: String::from(&author_cap[1]),
                };
                info!("{:#?}", &ret_val);
                Ok(Box::new(ret_val))
            }
            Err(why) => Err(why.into()),
        }
    }

    async fn get_title(&self) -> StreamResult<String> {
        Ok(self.description.clone())
    }

    async fn get_author(&self) -> StreamResult<String> {
        Ok(self.author.clone())
    }

    async fn is_online(&self) -> StreamResult<Status> {
        Ok(Status::Online)
    }

    async fn get_stream(&self) -> StreamResult<StreamType> {
        Ok(StreamType::Chuncked(
            self.client.get(&self.douyin_url).build()?,
        ))
    }

    async fn get_ext(&self) -> StreamResult<String> {
        Ok(String::from("mp4"))
    }

    async fn get_default_name(&self) -> StreamResult<String> {
        Ok(format!(
            "{}-{}-{}.{}",
            self.video_id,
            self.get_title().await?,
            self.get_author().await?,
            self.get_ext().await?
        ))
    }
}
