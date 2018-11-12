use Streamable;
use regex::Regex;

use utils::error::StreamError;
use utils::error::RsgetError;

use utils::downloaders::DownloadClient;

use std::fs::File;

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
    pub douyin_url: String, //DouyinRoot,
    pub description: String,
    pub author: String,
    client: DownloadClient,
}

impl Streamable for Douyin {
    fn new(url: String) -> Result<Box<Douyin>, StreamError> {
        let dc = DownloadClient::new()?;
        let site_req = dc.make_request(&url, None)?;
        let res: Result<String, StreamError> = dc.download_to_string(site_req);
        match res {
            Ok(some) => {
                let url_re: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?iesdouyin\.com/share/video/([a-zA-Z0-9]+)/.*")?;
                let video_re = Regex::new(r#"playAddr:\s*"(.+)""#)?;
                let description_re = Regex::new(r#"<p class="desc">([^<]*)</p>"#)?;
                let author_re = Regex::new(r#"<p class="name nowrap">@([^<]*)</p>"#)?;
                let id_cap = url_re
                    .captures(&url)
                    .ok_or_else(|| StreamError::Rsget(RsgetError::new("Regex did not find any video id")))?;
                let video_cap = video_re
                    .captures(&some)
                    .ok_or_else(|| StreamError::Rsget(RsgetError::new("Regex did not find any hostinfo")))?;
                let description_cap = description_re
                    .captures(&some)
                    .ok_or_else(|| StreamError::Rsget(RsgetError::new("Regex did not find any description")))?;
                let author_cap = author_re
                    .captures(&some)
                    .ok_or_else(|| StreamError::Rsget(RsgetError::new("Regex did not find any author")))?;

                let ret_val = Douyin {
                    client: dc,
                    url: url.clone(),
                    video_id: String::from(&id_cap[1]),
                    douyin_url: String::from(&video_cap[1]),
                    description: String::from(&description_cap[1]),
                    author: String::from(&author_cap[1]),
                };
                info!("{:#?}", &ret_val);
                Ok(Box::new(ret_val))
            },
            Err(why) => {
                Err(why)
            },
        }

    }

    fn get_title(&self) -> Option<String> {
        Some(self.description.clone())
    }

    fn get_author(&self) -> Option<String> {
        Some(self.author.clone())
    }

    fn is_online(&self) -> bool {
        true
    }

    fn get_stream(&self) -> String {
        self.douyin_url.clone()
    }

    fn get_ext(&self) -> String {
        String::from("mp4")
    }

    fn get_default_name(&self) -> String {
        format!(
            "{}-{}-{}.{}",
            self.video_id,
            self.get_title().unwrap(),
            self.get_author().unwrap(),
            self.get_ext()
        )
    }

    fn download(&self, path: String) -> Result<(), StreamError> {
        println!(
            "{} by {} ({})",
            self.get_title().unwrap(),
            self.get_author().unwrap(),
            self.video_id
        );
        self.client.download_to_file(
            &self.get_stream(),
            File::create(path)?,
            true,
        )
    }    
}
