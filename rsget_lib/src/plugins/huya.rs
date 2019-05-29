use crate::Streamable;
use regex::Regex;
use serde_json;

use stream_lib::StreamType;

use crate::utils::downloaders::DownloadClient;

use crate::utils::error::RsgetError;
use crate::utils::error::StreamError;

use chrono::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HuyaInfo {
    html5: i64,
    #[serde(rename = "WEBYYHOST")]
    webyyhost: String,
    #[serde(rename = "WEBYYSWF")]
    webyyswf: String,
    #[serde(rename = "WEBYYFROM")]
    webyyfrom: String,
    vappid: i64,
    stream: Option<HuyaStream>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct HuyaStream {
    status: i64,
    msg: String,
    data: Vec<Daum>,
    count: i64,
    #[serde(rename = "vMultiStreamInfo")]
    v_multi_stream_info: Vec<VMultiStreamInfo>,
    #[serde(rename = "iWebDefaultBitRate")]
    i_web_default_bit_rate: i64,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct Daum {
    #[serde(rename = "gameLiveInfo")]
    game_live_info: GameLiveInfo,
    #[serde(rename = "gameStreamInfoList")]
    game_stream_info_list: Vec<GameStreamInfoList>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct GameLiveInfo {
    uid: String,
    sex: String,
    #[serde(rename = "gameFullName")]
    game_full_name: String,
    #[serde(rename = "gameHostName")]
    game_host_name: String,
    #[serde(rename = "startTime")]
    start_time: String,
    #[serde(rename = "activityId")]
    activity_id: String,
    level: String,
    #[serde(rename = "totalCount")]
    total_count: String,
    #[serde(rename = "roomName")]
    room_name: String,
    #[serde(rename = "isSecret")]
    is_secret: String,
    #[serde(rename = "cameraOpen")]
    camera_open: String,
    #[serde(rename = "liveChannel")]
    live_channel: String,
    #[serde(rename = "bussType")]
    buss_type: String,
    yyid: String,
    screenshot: String,
    #[serde(rename = "activityCount")]
    activity_count: String,
    #[serde(rename = "privateHost")]
    private_host: String,
    #[serde(rename = "recommendStatus")]
    recommend_status: String,
    nick: String,
    #[serde(rename = "shortChannel")]
    short_channel: String,
    avatar180: String,
    gid: String,
    channel: String,
    introduction: String,
    #[serde(rename = "profileHomeHost")]
    profile_home_host: String,
    #[serde(rename = "liveSourceType")]
    live_source_type: String,
    #[serde(rename = "screenType")]
    screen_type: String,
    #[serde(rename = "bitRate")]
    bit_rate: String,
    #[serde(rename = "gameType")]
    game_type: ::serde_json::Value,
    #[serde(rename = "attendeeCount")]
    attendee_count: ::serde_json::Value,
    #[serde(rename = "multiStreamFlag")]
    multi_stream_flag: String,
    #[serde(rename = "codecType")]
    codec_type: String,
    #[serde(rename = "liveCompatibleFlag")]
    live_compatible_flag: String,
    #[serde(rename = "profileRoom")]
    profile_room: String,
    #[serde(rename = "liveId")]
    live_id: String,
    #[serde(rename = "recommendTagName")]
    recommend_tag_name: String,
    #[serde(rename = "contentIntro")]
    content_intro: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct GameStreamInfoList {
    #[serde(rename = "sCdnType")]
    s_cdn_type: String,
    #[serde(rename = "iIsMaster")]
    i_is_master: i64,
    #[serde(rename = "lChannelId")]
    l_channel_id: i64,
    #[serde(rename = "lSubChannelId")]
    l_sub_channel_id: i64,
    #[serde(rename = "lPresenterUid")]
    l_presenter_uid: i64,
    #[serde(rename = "sStreamName")]
    s_stream_name: String,
    #[serde(rename = "sFlvUrl")]
    s_flv_url: String,
    #[serde(rename = "sFlvUrlSuffix")]
    s_flv_url_suffix: String,
    #[serde(rename = "sFlvAntiCode")]
    s_flv_anti_code: String,
    #[serde(rename = "sHlsUrl")]
    s_hls_url: String,
    #[serde(rename = "sHlsUrlSuffix")]
    s_hls_url_suffix: String,
    #[serde(rename = "sHlsAntiCode")]
    s_hls_anti_code: String,
    #[serde(rename = "iLineIndex")]
    i_line_index: i64,
    #[serde(rename = "iIsMultiStream")]
    i_is_multi_stream: i64,
    #[serde(rename = "iPCPriorityRate")]
    i_pcpriority_rate: i64,
    #[serde(rename = "iWebPriorityRate")]
    i_web_priority_rate: i64,
    #[serde(rename = "iMobilePriorityRate")]
    i_mobile_priority_rate: i64,
    #[serde(rename = "vFlvIPList")]
    v_flv_iplist: Vec<::serde_json::Value>,
    #[serde(rename = "iIsP2PSupport")]
    i_is_p2_psupport: i64,
    #[serde(rename = "sP2pUrl")]
    s_p2_p_url: String,
    #[serde(rename = "sP2pUrlSuffix")]
    s_p2_p_url_suffix: String,
    #[serde(rename = "sP2pAntiCode")]
    s_p2_p_anti_code: String,
    #[serde(rename = "lFreeFlag")]
    l_free_flag: i64,
    #[serde(rename = "newCFlvAntiCode")]
    new_cflv_anti_code: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct VMultiStreamInfo {
    #[serde(rename = "sDisplayName")]
    s_display_name: String,
    #[serde(rename = "iBitRate")]
    i_bit_rate: i64,
}

#[derive(Clone, Debug)]
pub struct Huya {
    client: DownloadClient,
    pub url: String,
    pub room_id: String,
    host_info: HuyaInfo,
}

impl Streamable for Huya {
    fn new(url: String) -> Result<Box<Huya>, StreamError> {
        let dc = DownloadClient::new()?;

        let room_id_re = Regex::new(r"/([a-zA-Z0-9]+)$")?;
        let cap = match room_id_re.captures(&url) {
            Some(capture) => capture,
            None => return Err(StreamError::Rsget(RsgetError::new("URL capture failed"))),
        };
        let site_url = format!("http://huya.com/{}", &cap[1]);
        let site_req = dc.make_request(&site_url, None)?;
        let res: Result<String, StreamError> = dc.download_to_string(site_req);
        match res {
            Ok(some) => {
                let hostinfo_re =
                    Regex::new(r"hyPlayerConfig = ([^<]*);\s*window\.TT_LIVE_TIMING")?;
                let hi_cap = hostinfo_re.captures(&some).ok_or_else(|| {
                    StreamError::Rsget(RsgetError::new("Regex did not find any hostinfo"))
                })?;
                let hi: HuyaInfo = match serde_json::from_str(&hi_cap[1]) {
                    Ok(info) => info,
                    Err(why) => return Err(StreamError::Json(why)),
                };
                let xy = Huya {
                    client: dc,
                    url: url.clone(),
                    room_id: String::from(&cap[1]),
                    host_info: hi,
                };
                debug!("{:#?}", &xy);
                Ok(Box::new(xy))
            }
            Err(why) => Err(why),
        }
    }

    fn get_title(&self) -> Option<String> {
        Some(
            self.host_info.stream.clone()?.data[0]
                .game_live_info
                .introduction
                .clone(),
        )
    }

    fn get_author(&self) -> Option<String> {
        Some(
            self.host_info.stream.clone()?.data[0]
                .game_live_info
                .nick
                .clone(),
        )
    }

    fn is_online(&self) -> bool {
        self.host_info.stream.is_some()
    }

    /// Hls stream
    fn get_stream(&self) -> Result<StreamType, StreamError> {
        let stream_url = format!(
            "{}/{}.{}?{}",
            &self
                .host_info
                .stream
                .clone()
                .unwrap()
                .data
                .get(0)
                .ok_or(RsgetError::Offline)?
                .game_stream_info_list
                .get(0)
                .ok_or(RsgetError::Offline)?
                .s_hls_url,
            &self
                .host_info
                .stream
                .clone()
                .unwrap()
                .data
                .get(0)
                .ok_or(RsgetError::Offline)?
                .game_stream_info_list
                .get(0)
                .ok_or(RsgetError::Offline)?
                .s_stream_name,
            &self
                .host_info
                .stream
                .clone()
                .unwrap()
                .data
                .get(0)
                .ok_or(RsgetError::Offline)?
                .game_stream_info_list
                .get(0)
                .ok_or(RsgetError::Offline)?
                .s_hls_url_suffix,
            &self
                .host_info
                .stream
                .clone()
                .unwrap()
                .data
                .get(0)
                .ok_or(RsgetError::Offline)?
                .game_stream_info_list
                .get(0)
                .ok_or(RsgetError::Offline)?
                .s_hls_anti_code
        );
        println!("URL: {}", stream_url);
        Ok(StreamType::HLS(
            self.client.rclient.get(&stream_url).build()?,
        ))
    }

    fn get_ext(&self) -> String {
        String::from("mp4")
    }

    fn get_default_name(&self) -> String {
        let local: DateTime<Local> = Local::now();
        format!(
            "{}-{:04}-{:02}-{:02}-{:02}-{:02}-{}-{}.{}",
            self.room_id,
            local.year(),
            local.month(),
            local.day(),
            local.hour(),
            local.minute(),
            self.get_author().unwrap(),
            self.get_title().unwrap(),
            self.get_ext()
        )
    }
    fn get_reqwest_client(&self) -> &reqwest::Client {
        &self.client.rclient
    }
}
