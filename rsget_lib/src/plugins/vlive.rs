#![allow(unused)]

use crate::{Status, Streamable};
use regex::Regex;

use async_trait::async_trait;

use stream_lib::DownloadStream;

use crate::utils::error::StreamResult;

use crate::utils::error::RsgetError;
use crate::utils::error::StreamError;

#[derive(Debug, Clone)]
pub struct Vlive {
    http: reqwest::Client,
    url: String,
    title: String,
    author: String,
    video_url: Option<String>,
    // TODO FOR ERK: This field is currently unused. This is due to Rsgets design being too focused on making plugin
    // implementation easier for developers, but at the expense of more "native" per site support. To access the m3u8
    // files and the .ts files from vlive you need to provide a session key for the requests. If you look at where I73
    
    // define VideoInfo, theres is a list field `streams`. Each of these streams has field "key" which has a name and
    // a value, which must be appended as a url parameter to every request to that stream. For example:
    // {
    //   "type": "HLS",
    //   "key": {
    //     "type": "param",
    //     "name": "__gda__",
    //     "value": "1568079266_45ce9b5ed235565dae29acec5cf9d26f"
    //   },
    //   "source": "https://globalv-rmcnmv.akamaized.net/c/read/v2/VOD_ALPHA/global_v_2019_09_08_228/hls/97753681-d218-11e9-bed2-246e963a41ed.m3u8"
    // }
    //
    // means we must append `?__gda__=156807...26f` to the end of that m3u8. You might think that's not too bad? I actually
    // already do that in my code, but as it turns out, the code must **also** be appended to every url in that m3u8 file.
    // this is where the weakness of Rsgets design comes in. In `get_stream` you can only return a HLS, Chunked or
    // NamedPlaylist, which none (as far as I can see) can be used after you have manually parsed and edited the M3U file.
    // Even if you managed to edit this m3u8 file, all the listed .ts files must have the key too! All in all, it means
    // that the vlive backend cannot use the recommended stream endpoint but rather relies on manually choosing one of
    // the `videos` entries, which I think might not be available for every VOD/Stream
    #[allow(unused)]
    stream_url: Option<String>,
}

#[async_trait]
impl Streamable for Vlive {
    async fn new(url: String) -> StreamResult<Box<Vlive>> {
        let http = reqwest::Client::new();

        let page_req = http.get(&url).send().await?;
        let page = page_req.text().await?;

        // The session key and video ID (not the short video seq id at the end of url) are parsed from js
        let vid_id_re = Regex::new(r#"vlive\.video\.init\((\s*"(.*?)",\s*?){6}"#)?;
        let vid_key_re = Regex::new(r#"vlive\.video\.init\((\s*"(.*?)",\s*?){7}"#)?;
        let vid_chan_re = Regex::new(r#"gaCname\s*:\s*"(.*?)""#)?;

        let id = vid_id_re
            .captures(&page)
            .ok_or_else(|| StreamError::Rsget(RsgetError::new("No capture found")))?[2]
            .to_string();
        let key = vid_key_re
            .captures(&page)
            .ok_or_else(|| StreamError::Rsget(RsgetError::new("No capture found")))?[2]
            .to_string();
        let chan = vid_chan_re
            .captures(&page)
            .ok_or_else(|| StreamError::Rsget(RsgetError::new("No capture found")))?[1]
            .to_string();

        let page_req = http.get(&format!("https://global.apis.naver.com/rmcnmv/rmcnmv/vod_play_videoInfo.json?key={}&videoId={}", key, id)).send().await?;

        // all these structs are quite excessive for what we actually need but i want to be ready for the "Quality Update"
        // Currently this backend just chooses the video with the highest file size, aka most likely to be highest quality
        #[derive(Debug, Deserialize)]
        struct VideoInfo {
            meta: Meta,
            videos: Videos,
            streams: Vec<Stream>,
        }
        #[derive(Debug, Deserialize)]
        struct Meta {
            subject: String,
        }
        #[derive(Debug, Deserialize)]
        struct Videos {
            list: Vec<Video>,
        }
        #[derive(Debug, Deserialize)]
        struct Video {
            source: String,
            size: usize,
            #[serde(rename = "encodingOption")]
            encoding_option: Quality,
            bitrate: Bitrate,
        }
        #[derive(Debug, Deserialize)]
        struct Quality {
            name: String,
            profile: H264,
            width: usize,
            height: usize,
        }
        #[derive(Debug, Deserialize)]
        struct Bitrate {
            video: f64,
            audio: f64,
        }
        #[derive(Debug, Deserialize)]
        #[serde(rename_all = "UPPERCASE")]
        enum H264 {
            Base,
            Main,
            High,
        }
        #[derive(Debug, Deserialize)]
        struct Stream {
            key: Key,
            source: String,
        }
        #[derive(Debug, Deserialize)]
        struct Key {
            name: String,
            value: String,
        }

        let info = page_req.json::<VideoInfo>().await?;
        let stream_url = info
            .streams
            .get(0)
            .map(|stream| format!("{}?{}={}", stream.source, stream.key.name, stream.key.value));

        let mut videos = info.videos.list;
        videos.sort_by_key(|video| video.size);
        let video_url = videos.last().map(|video| video.source.clone());

        Ok(Box::new(Vlive {
            http,
            url,
            title: info.meta.subject,
            author: chan,
            video_url,
            stream_url,
        }))
    }

    async fn get_title(&self) -> StreamResult<String> {
        Ok(self.title.clone())
    }

    async fn get_author(&self) -> StreamResult<String> {
        Ok(self.author.clone())
    }

    async fn is_online(&self) -> StreamResult<Status> {
        Ok(Status::Online)
    }

    async fn get_stream(&self) -> StreamResult<DownloadStream> {
        // READ TODO At the beginning
        // let url = self.stream_url.clone().ok_or(StreamError::Rsget(RsgetError::new("No streams available")))?;
        // Ok(StreamType::HLS(self.client.make_request(&url, None)?))

        let url = self
            .video_url
            .clone()
            .ok_or_else(|| StreamError::Rsget(RsgetError::new("No videos available")))?;
        Ok(stream_lib::download_chunked(
            self.http.clone(),
            self.http.get(url).build()?,
        ))
    }

    async fn get_ext(&self) -> StreamResult<String> {
        Ok("mp4".into())
    }

    async fn get_default_name(&self) -> StreamResult<String> {
        Ok(format!(
            "{}-{}.{}",
            self.author,
            self.title,
            self.get_ext().await?
        ))
    }
}
