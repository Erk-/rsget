use regex::Regex;
use Streamable;
use utils::error::StreamError;
use utils::error::RsgetError;
use HttpsClient;
use plugins::{douyu, panda, xingyan, xingyan2, inke/*, afreeca*/};
// Option<Box<Streamable + 'static>>
pub fn get_site(client: &HttpsClient, input: &str) -> Result<Box<Streamable>, StreamError>
{
    let re_xingyan_panda: Regex = Regex::new(r"^(?:https?://)?xingyan\.panda\.tv/[0-9]+/?").unwrap();
    let re_panda: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?panda\.tv/[0-9]+/?").unwrap();
    let re_douyu: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?douyu\.com/[a-zA-Z0-9]+/?").unwrap();
    // let re_afreeca: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?(?:play\.)?afreecatv.com/[a-zA-Z0-9]+/?(?:/[0-9]+)?").unwrap();
    let re_inke: Regex = Regex::new(r"^(?:https?://)?(?:www\.)?inke\.cn/live\.html\?uid=[0-9]+").unwrap();
    
    match input {
        url if re_panda.is_match(url) => {
            match panda::PandaTv::new(client, String::from(url)) {
                Ok(s) => Ok(s),
                Err(why) => Err(why),
            }
        },
        url if re_xingyan_panda.is_match(url) => {
            match xingyan::Xingyan::new(client, String::from(url)) {
                Ok(s) => Ok(s),
                Err(_why) => {
                    match xingyan2::Xingyan2::new(client, String::from(url)) {
                        Ok(s) => Ok(s),
                        Err(why) => Err(why),
                    }
                },
            }
        },
        url if re_douyu.is_match(url) => {
            match douyu::Douyu::new(client, String::from(url))  {
                Ok(s) => Ok(s),
                Err(why) => Err(why),
            }
        },

        // url if re_afreeca.is_match(url) => {
        //     match afreeca::Afreeca::new(client, String::from(url))  {
        //         Ok(s) => Ok(s),
        //         Err(why) => Err(why),
        //     }
        // },
        url if re_inke.is_match(url) => {
            match inke::Inke::new(client, String::from(url)) {
                Ok(s) => Ok(s),
                Err(why) => Err(why),
            }
        },      
        _ => Err(StreamError::Rsget(RsgetError::new("Site not supported."))),
    }
}
