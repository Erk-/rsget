use regex::Regex;
use Streamable;
use plugins::{panda, xingyan};

pub fn get_site(input: &str) -> Option<Box<Streamable + 'static>> {
    lazy_static! {
        static ref RE_XINGYAN_PANDA: Regex =
            Regex::new(r"^(?:https?://)?xingyan\.panda\.tv/[0-9]+").unwrap();
        static ref RE_PANDA: Regex =
            Regex::new(r"^(?:https?://)?(?:www\.)?panda\.tv/[0-9]+").unwrap();
    }

    match input {
        url if RE_PANDA.is_match(url) => {
            Some(Box::new(panda::PandaTv::new(String::from(url))))
        },
        url if RE_XINGYAN_PANDA.is_match(url) => {
            Some(Box::new(xingyan::Xingyan::new(String::from(url))))
        },
        _ => None,
    }
}
