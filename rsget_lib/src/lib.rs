#[macro_use] extern crate log;
extern crate reqwest;
extern crate regex;
extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate indicatif; 
extern crate chrono;

#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

pub trait Downloadable {
    fn new(url: String) -> Self;
    fn get_title(&self) -> Option<String>;
    fn get_author(&self) -> Option<String>;
    fn get_size(&self) -> Option<String>;
    fn get_stream(&self) -> String;
}

pub trait Streamable {
    fn new(url: String) -> Self;
    fn get_title(&self) -> Option<String>;
    fn get_author(&self) -> Option<String>;
    //fn get_stream(&self) -> <T: Stream>
    fn is_online(&self) -> bool;
    fn get_stream(&self) -> String;
    fn get_ext(&self) -> String;
    fn get_default_name(&self) -> String;
    fn download(&self, path: String) -> Option<()>;
}
mod utils;
pub mod plugins;
