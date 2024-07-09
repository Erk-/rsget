/*
use crate::plugins::{
    afreeca::Afreeca, dlive::DLive, douyin::Douyin, douyu::Douyu, huya::Huya, inke::Inke,
    mixer::Mixer, tiktok::TikTok, twitch::Twitch, vlive::Vlive,
};
*/
//use crate::plugins::{Afreeca, Bilibili, DLive, Twitch, Vlive};
use crate::plugins::{Afreeca, Bilibili, DLive, Drdk, Twitch, Vlive};
use crate::utils::error::RsgetError;
use crate::utils::error::StreamError;
use crate::utils::error::StreamResult;
use crate::Streamable;
use regex::Regex;

pub async fn get_site(input: &str) -> StreamResult<Box<dyn Streamable + Send>> {
    match _get_site(input).await {
        Ok(s) => Ok(s),
        Err(StreamError::Rsget(_)) => {
            let res = reqwest::get(input).await?;
            let final_url = res.url().as_str();
            _get_site(final_url).await
        }
        Err(why) => Err(why),
    }
}

async fn _get_site(input: &str) -> StreamResult<Box<dyn Streamable + Send>> {
    let re_drdk: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?dr\.dk/drtv/kanal/[a-zA-Z0-9-_]+")?;
    let re_afreeca: Regex = Regex::new(
        r"^(?:https?://)?(?:www\.)?(?:play\.)?afreecatv.com/[a-zA-Z0-9]+/?(?:/[0-9]+)?",
    )?;
    let re_dlive = Regex::new(r"^(?:https?://)?(?:www\.)?dlive\.tv/[a-zA-Z0-9]+\??.*")?;
    let re_twitch = Regex::new(r"^(?:https?://)?(?:www\.)?twitch\.tv/([a-zA-Z0-9_]+)")?;
    let re_bilibili = Regex::new(r"^(?:https?://)?(?:www\.)?live\.bilibili\.com/([0-9]+)")?;
    //let re_douyu: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?douyu\.com/[a-zA-Z0-9]+/?")?;
    /*
    let re_inke: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?inke\.cn/live\.html\?uid=[0-9]+")?;
    let re_douyin: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?iesdouyin\.com/.*")?;
    let re_tiktok: Regex =
        Regex::new(r"^(?:https?://)?(?:www\.)?(?:m\.)?tiktok\.com/v/(?:[a-zA-Z0-9]+)(?:\.html)?")?;
    let re_huya: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?huya\.com/[a-zA-Z0-9]+")?;*/
    let re_vlive: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?vlive\.tv/video/(\d+)")?;
    match input {
        //url if re_douyu.is_match(url) => Ok(Douyu::new(String::from(url))?),
        url if re_afreeca.is_match(url) => Ok(Afreeca::new(String::from(url)).await?),
        url if re_bilibili.is_match(url) => Ok(Bilibili::new(String::from(url)).await?),
        url if re_dlive.is_match(url) => Ok(DLive::new(String::from(url)).await?),
        url if re_drdk.is_match(url) => Ok(Drdk::new(String::from(url)).await?),
        url if re_twitch.is_match(url) => Ok(Twitch::new(String::from(url)).await?),
        url if re_vlive.is_match(url) => Ok(Vlive::new(String::from(url)).await?),
        /*
        url if re_inke.is_match(url) => Ok(Inke::new(String::from(url))?),
        url if re_douyin.is_match(url) => Ok(Douyin::new(String::from(url)).await?),
        url if re_tiktok.is_match(url) => Ok(TikTok::new(String::from(url))?),
        url if re_huya.is_match(url) => Ok(Huya::new(String::from(url))?),
        url if re_twitch.is_match(url) => Ok(Twitch::new(String::from(url))?),
        url if re_dlive.is_match(url) => Ok(DLive::new(String::from(url))?),

        */
        _ => Err(StreamError::Rsget(RsgetError::new("Site not supported."))),
    }
}
