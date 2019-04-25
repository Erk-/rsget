use crate::Streamable;
use regex::Regex;
use serde_json;
use serde_json::Value;

use stream_lib::StreamType;

use crate::utils::error::StreamError;
use crate::utils::error::RsgetError;

use crate::utils::downloaders::DownloadClient;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Author {
  pub uid: String,
  pub avatar_larger: LabelTop,
  pub birthday: String,
  pub custom_verify: String,
  pub is_verified: bool,
  pub nickname: String,
  pub user_mode: i64,
  pub short_id: String,
  pub hide_location: bool,
  pub gender: i64,
  pub secret: i64,
  pub user_period: i64,
  pub avatar_medium: LabelTop,
  pub signature: String,
  pub avatar_thumb: LabelTop,
  pub weibo_verify: String,
  pub unique_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Comments {
  pub status: i64,
  pub from_author: bool,
  #[serde(skip_serializing)]
  pub reply_comment: Vec<String>,
  pub text: String,
  pub cid: String,
  pub digg_count: i64,
  #[serde(skip_serializing)]
  pub text_extra: Vec<String>,
  pub create_time: i64,
  pub reply_id: String,
  pub user: User,
  pub aweme_id: String,
  pub user_digged: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LabelTop {
  pub url_list: Vec<String>,
  pub uri: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Music {
  pub cover_hd: LabelTop,
  pub status: i64,
  pub owner_nickname: String,
  pub user_count: i64,
  pub is_video_self_see: bool,
  pub title: String,
  pub play_url: LabelTop,
  pub owner_id: String,
  #[serde(skip_serializing)]
  pub app_unshelve_info: Value,
  pub mid: String,
  pub author_name: String,
  pub schema_url: String,
  pub is_only_owner_use: bool,
  pub source: i64,
  pub cover_large: LabelTop,
  pub owner_handle: String,
  pub is_del_video: bool,
  pub cover_thumb: LabelTop,
  pub cover_medium: LabelTop,
  pub music_name: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PlayAddr {
  pub url_list: Vec<String>,
  pub url_key: String,
  pub uri: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PolicyVersion {
  #[serde(rename = "GLOBAL")]
  pub _global: Option<i64>,
  #[serde(rename = "SE")]
  pub _se: Option<i64>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct RiskInfos {
  pub warn: bool,
  pub content: String,
  pub risk_sink: bool,
  #[serde(rename = "type")]
  pub _type: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TikTokRoot {
  pub risk_infos: RiskInfos,
  pub label_top: LabelTop,
  pub author_user_id: i64,
  pub item_comment_settings: i64,
  pub rate: i64,
  pub create_time: i64,
  pub video: Video,
  pub comments: Vec<Comments>,
  pub aweme_id: String,
  #[serde(skip_serializing)]
  pub video_labels: Vec<String>,
  pub is_vr: bool,
  pub vr_type: i64,
  pub statistics: Statistics,
  pub author: Author,
  pub prevent_download: bool,
  pub cmt_swt: bool,
  pub share_url: String,
  pub is_ads: bool,
  pub comment_count: i64,
  pub music: Music,
  pub bodydance_score: i64,
  pub xigua_task: XiguaTask,
  pub is_hash_tag: i64,
  pub status: Status,
  pub sort_label: String,
  pub share_info: ShareInfo,
  #[serde(skip_serializing)]
  pub video_text: Vec<String>,
  pub is_top: i64,
  pub aweme_type: i64,
  pub desc: String,
  pub group_id: String,
  #[serde(skip_serializing)]
  pub geofencing: Vec<String>,
  pub region: String,
  pub is_pgcshow: bool,
  pub is_relieve: bool,
  pub text_extra: Vec<TextExtra>,
  pub user_digged: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct ShareInfo {
  pub share_weibo_desc: String,
  pub bool_persist: i64,
  pub share_quote: String,
  pub share_title: String,
  pub share_signature_desc: String,
  pub share_signature_url: String,
  pub share_link_desc: String,
  pub share_url: String,
  pub share_desc: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Statistics {
  pub comment_count_str: String,
  pub digg_count_str: String,
  pub forward_count: i64,
  pub digg_count: i64,
  pub share_count_str: String,
  pub play_count: i64,
  pub comment_count: i64,
  pub aweme_id: String,
  pub share_count: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Status {
  pub private_status: i64,
  pub reviewed: i64,
  pub is_prohibited: bool,
  pub with_goods: bool,
  pub is_private: bool,
  pub download_status: i64,
  pub is_delete: bool,
  pub with_fusion_goods: bool,
  pub self_see: bool,
  pub in_reviewing: bool,
  pub allow_share: bool,
  pub allow_comment: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TextExtra {
  pub start: i64,
  pub end: i64,
  pub hashtag_name: String,
  #[serde(rename = "type")]
  pub _type: i64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct User {
  pub youtube_channel_title: String,
  pub share_qrcode_uri: String,
  pub app_id: i64,
  pub original_music_qrcode: Option<String>,
  pub is_gov_media_vip: bool,
  pub live_commerce: bool,
  pub account_region: String,
  pub user_period: i64,
  pub reflow_page_gid: i64,
  pub is_binded_weibo: bool,
  #[serde(rename = "video_icon_virtual_URI")]
  pub video_icon_virtual_uri: String,
  pub risk_flag: i64,
  pub school_name: String,
  pub download_setting: i64,
  pub cv_level: String,
  pub custom_verify: String,
  pub special_lock: i64,
  pub user_canceled: bool,
  pub shield_comment_notice: i64,
  #[serde(skip_serializing)]
  pub type_label: Vec<String>,
  pub hide_location: bool,
  pub gender: i64,
  pub video_icon: LabelTop,
  pub school_poi_id: String,
  pub live_agreement: i64,
  pub is_phone_binded: bool,
  pub prevent_download: bool,
  pub weibo_schema: String,
  pub create_time: i64,
  pub has_insights: bool,
  pub react_setting: i64,
  pub google_account: String,
  pub community_discipline_status: i64,
  pub user_mode: i64,
  pub need_recommend: i64,
  pub update_before: i64,
  pub has_register_notice: i64,
  pub room_id: i64,
  pub avatar_medium: LabelTop,
  pub has_orders: bool,
  pub reflow_page_uid: i64,
  pub cover_url: Vec<LabelTop>,
  pub duet_setting: i64,
  pub language: String,
  #[serde(skip_serializing)]
  pub geofencing: Vec<String>,
  pub ins_id: String,
  pub unique_id_modify_time: i64,
  pub school_type: i64,
  pub twitter_name: String,
  pub avatar_uri: String,
  pub signature: String,
  pub weibo_verify: String,
  pub comment_setting: i64,
  pub with_fusion_shop_entry: bool,
  pub youtube_channel_id: String,
  pub avatar_larger: LabelTop,
  pub enterprise_verify_reason: String,
  pub user_rate: i64,
  pub live_verify: i64,
  pub short_id: String,
  pub secret: i64,
  pub avatar_thumb: LabelTop,
  pub is_verified: bool,
  pub hide_search: bool,
  pub with_commerce_entry: bool,
  pub download_prompt_ts: i64,
  pub twitter_id: String,
  pub has_email: bool,
  #[serde(skip_serializing)]
  pub policy_version: PolicyVersion,
  pub region: String,
  pub uid: String,
  pub bind_phone: String,
  pub weibo_url: String,
  pub live_agreement_time: i64,
  pub weibo_name: String,
  pub commerce_user_level: i64,
  pub verify_info: String,
  pub apple_account: i64,
  pub accept_private_policy: bool,
  pub shield_digg_notice: i64,
  pub verification_type: i64,
  pub neiguang_shield: i64,
  pub live_rec_level: i64,
  pub authority_status: i64,
  pub enterprise_verify: bool,
  pub birthday: String,
  pub is_ad_fake: bool,
  pub nickname: String,
  pub shield_follow_notice: i64,
  pub original_music_cover: Option<String>,
  pub creator_level: i64,
  pub nickname_lock: i64,
  pub status: i64,
  pub unique_id: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Video {
  pub ratio: String,
  pub origin_cover: LabelTop,
  pub play_addr: PlayAddr,
  pub cover: LabelTop,
  pub height: i64,
  pub width: i64,
  pub download_addr: PlayAddr,
  pub has_watermark: bool,
  pub play_addr_lowbr: PlayAddr,
  pub dynamic_cover: LabelTop,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct XiguaTask {
  pub is_xigua_task: bool,
}

#[derive(Clone, Debug)]
pub struct TikTok {
    pub url: String,
    pub video_id: String,
    pub tiktok: TikTokRoot,
    client: DownloadClient,
}

impl Streamable for TikTok {
    fn new(url: String) -> Result<Box<TikTok>, StreamError> {
        let dc = DownloadClient::new()?;
        let site_req = dc.make_request(&url, None)?;
        let res: String = dc.download_to_string(site_req)?;

        let url_re = Regex::new(r"^(?:https?://)?(?:www\.)?(?:m\.)?tiktok\.com/v/([a-zA-Z0-9]+)(?:\.html)?")?;
        let json_re = Regex::new(r"var data = (\{.+\});")?;
        let id_cap = url_re
            .captures(&url)
            .ok_or_else(|| StreamError::Rsget(RsgetError::new("[TIKTOK] Regex did not find any video id")))?;
        let json_cap = json_re
            .captures(&res)
            .ok_or_else(|| StreamError::Rsget(RsgetError::new("[TIKTOK] Regex did not find any json")))?;
        debug!("{}", &json_cap[1]);
        let tik: TikTokRoot = serde_json::from_str(&json_cap[1])?;
        
        let ret_val = TikTok {
            client: dc,
            url: url.clone(),
            video_id: String::from(&id_cap[1]),
            tiktok: tik,
        };
        info!("{:#?}", &ret_val);
        Ok(Box::new(ret_val))
    }

    fn get_title(&self) -> Option<String> {
        Some(self.tiktok.desc.clone())
    }

    fn get_author(&self) -> Option<String> {
        Some(self.tiktok.author.nickname.clone())
    }

    fn is_online(&self) -> bool {
        true
    }

    fn get_stream(&self) -> Result<StreamType, StreamError> {
        Ok(StreamType::Chuncked(self.client.rclient.get(
            &self.tiktok.video.download_addr.url_list[0]
        ).build()?))
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
    fn get_reqwest_client(&self) -> &reqwest::Client {
        &self.client.rclient
    }
}
