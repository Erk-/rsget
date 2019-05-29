#![allow(clippy::new_ret_no_self)]
#![deny(rust_2018_idioms)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

use crate::utils::error::RsgetError;
use crate::utils::error::StreamError;

use std::boxed::Box;
use std::io::Write;

use stream_lib::Stream;
use stream_lib::StreamType;

use reqwest::Client as ReqwestClient;

pub trait Streamable {
    /// Creates a new streamable
    fn new(url: String) -> Result<Box<Self>, StreamError>
    where
        Self: Sized + Sync;
    /// Returns the title of the stream if possible
    fn get_title(&self) -> Option<String>;
    /// Returns the author of the stream if possible
    fn get_author(&self) -> Option<String>;
    /// Returns if the stream is online
    fn is_online(&self) -> bool;
    /// Gets the url of the stream
    fn get_stream(&self) -> Result<StreamType, StreamError>;
    /// Returns what extension the stream should be
    fn get_ext(&self) -> String;
    /// Gets the default name of the stream
    fn get_default_name(&self) -> String;
    fn get_reqwest_client(&self) -> &ReqwestClient {
        Box::leak(Box::new(ReqwestClient::new()))
    }
    /// Downloads the stream to a file
    fn download(&self, writer: Box<dyn Write>) -> Result<u64, StreamError> {
        if !self.is_online() {
            Err(StreamError::Rsget(RsgetError::Offline))
        } else {
            let stream = Stream::new(self.get_stream()?);
            Ok(stream.write_file(self.get_reqwest_client(), writer)?)
        }
    }
}

// impl<S> Streamable for Box<S>
// where S: Streamable
// { }

pub mod plugins;
pub mod utils;
