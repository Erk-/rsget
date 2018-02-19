use regex::Regex;
use Streamable;
use plugins::{douyu, panda, xingyan};

pub fn get_site(input: &str) -> Option<Box<Streamable + 'static>> {
    let re_xingyan_panda: Regex = Regex::new(r"^(?:https?://)?xingyan\.panda\.tv/[0-9]+").unwrap();
    let re_panda: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?panda\.tv/[0-9]+").unwrap();
    let re_douyu: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?douyu\.com/[a-zA-Z0-9]+").unwrap();

    match input {
        url if re_panda.is_match(url) => Some(Box::new(panda::PandaTv::new(String::from(url)))),
        url if re_xingyan_panda.is_match(url) => {
            Some(Box::new(xingyan::Xingyan::new(String::from(url))))
        }
        url if re_douyu.is_match(url) => Some(Box::new(douyu::Douyu::new(String::from(url)))),
        _ => None,
    }
}
