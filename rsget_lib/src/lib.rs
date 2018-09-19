extern crate chrono;
extern crate futures;
extern crate indicatif;
#[macro_use]
extern crate log;
extern crate md5;
extern crate regex;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate serde_urlencoded;
extern crate tokio;
extern crate http;
extern crate url;
extern crate hls_m3u8;
extern crate reqwest;

use utils::error::StreamError;
use utils::downloaders::DownloadClient;

pub trait Streamable {
    /// Creates a new streamable
    fn new(client: &DownloadClient, url: String) -> Result<Box<Self>, StreamError>
    where
        Self: Sized;
    /// Returns the title of the stream if possible
    fn get_title(&self) -> Option<String>;
    /// Returns the author of the stream if possible
    fn get_author(&self) -> Option<String>;
    //fn get_stream(&self) -> <T: Stream>
    /// Returns if the stream is online
    fn is_online(&self) -> bool;
    /// Gets the url of the stream
    fn get_stream(&self) -> String; // May be rewritten to no longer be a string but a enum to differentiate between types of stream
    /// Returns what extension the stream should be
    fn get_ext(&self) -> String;
    /// Gets the default name of the stream
    fn get_default_name(&self) -> String;
    /// Downloads the stream to a file
    fn download(&self, path: String) -> Result<(), StreamError>;
}

pub trait Stream {
    fn download(&self) -> Result<(), StreamError>;
}

pub mod utils;
pub mod plugins;
