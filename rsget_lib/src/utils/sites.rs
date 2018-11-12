use regex::Regex;
use Streamable;
use utils::error::StreamError;
use utils::error::RsgetError;
use plugins::{douyu::Douyu, panda::PandaTv, xingyan::Xingyan, inke::Inke, afreeca::Afreeca, douyin::Douyin};
// Option<Box<Streamable + 'static>>


pub fn get_site(input: &str) -> Result<Box<Streamable>, StreamError>
{
    let re_xingyan_panda: Regex = Regex::new(r"^(?:https?://)?xingyan\.panda\.tv/[0-9]+/?")?;
    let re_panda: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?panda\.tv/[0-9]+/?")?;
    let re_douyu: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?douyu\.com/[a-zA-Z0-9]+/?")?;
    let re_afreeca: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?(?:play\.)?afreecatv.com/[a-zA-Z0-9]+/?(?:/[0-9]+)?")?;
    let re_inke: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?inke\.cn/live\.html\?uid=[0-9]+")?;
    let re_douyin: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?(?:v\.)?douyin\.com/(?:[a-zA-Z0-9]+)")?;
    match input {
        url if re_panda.is_match(url) => {
            Ok(PandaTv::new(String::from(url))?)
        },
        url if re_xingyan_panda.is_match(url) => {
            Ok(Xingyan::new(String::from(url))?)
        },
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
        _ => Err(StreamError::Rsget(RsgetError::new("Site not supported."))),
    }
}
