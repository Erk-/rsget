use regex::Regex;
use crate::Streamable;
use crate::utils::error::StreamError;
use crate::utils::error::RsgetError;
use crate::plugins::{
    douyu::Douyu,
    inke::Inke,
    afreeca::Afreeca,
    douyin::Douyin,
    tiktok::TikTok,
    huya::Huya,
    dlive::DLive,
    twitch::Twitch,
};

use reqwest;

pub fn get_site(input: &str) -> Result<Box<Streamable>, StreamError> {
    match _get_site(input) {
        Ok(s) => Ok(s),
        Err(StreamError::Rsget(_)) => {
            let res = reqwest::get(input)?;
            let final_url = res.url().as_str();
            _get_site(final_url)
        },
        Err(why) => Err(why),
    }
}

fn _get_site(input: &str) -> Result<Box<Streamable>, StreamError> {
    let re_douyu: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?douyu\.com/[a-zA-Z0-9]+/?")?;
    let re_afreeca: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?(?:play\.)?afreecatv.com/[a-zA-Z0-9]+/?(?:/[0-9]+)?")?;
    let re_inke: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?inke\.cn/live\.html\?uid=[0-9]+")?;
    let re_douyin: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?iesdouyin\.com/.*")?;
    let re_tiktok: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?(?:m\.)?tiktok\.com/v/(?:[a-zA-Z0-9]+)(?:\.html)?")?;
    let re_huya: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?huya\.com/[a-zA-Z0-9]+")?;
    let re_dlive: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?dlive\.tv/[a-zA-Z0-9]+")?;
    let re_twitch: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?twitch\.tv/([a-zA-Z0-9]+)")?;
    match input {
        url if re_douyu.is_match(url) => {
            Ok(Douyu::new(String::from(url))?)
        },
        url if re_afreeca.is_match(url) => {
            Ok(Afreeca::new(String::from(url))?)
        },
        url if re_inke.is_match(url) => {
            Ok(Inke::new(String::from(url))?)
        },
        url if re_douyin.is_match(url) => {
            Ok(Douyin::new(String::from(url))?)
        },
        url if re_tiktok.is_match(url) => {
            Ok(TikTok::new(String::from(url))?)
        },
        url if re_huya.is_match(url) => {
            Ok(Huya::new(String::from(url))?)
        },
        url if re_dlive.is_match(url) => {
            Ok(DLive::new(String::from(url))?)
        },
        url if re_twitch.is_match(url) => {
            Ok(Twitch::new(String::from(url))?)
        },
        _ => Err(StreamError::Rsget(RsgetError::new("Site not supported."))),
    }
}
