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

use async_trait::async_trait;

/// Status of the live stream
pub enum Status {
    /// Stream is online.
    Online,
    /// Stream is offline.
    Offline,
    /// The status of the stream could not be determined.
    Unknown,
}

#[async_trait]
pub trait Streamable {
    /// Creates a new streamable
    async fn new(url: String) -> Result<Box<Self>, StreamError>
    where
        Self: Sized + Sync;
    /// Returns the title of the stream if possible
    async fn get_title(&self) -> Result<String, StreamError>;
    /// Returns the author of the stream if possible
    async fn get_author(&self) -> Result<String, StreamError>;
    /// Returns if the stream is online
    async fn is_online(&self) -> Result<Status, StreamError>;
    /// Gets the url of the stream
    async fn get_stream(&self) -> Result<StreamType, StreamError>;
    /// Returns what extension the stream should be
    async fn get_ext(&self) -> Result<String, StreamError>;
    /// Gets the default name of the stream
    async fn get_default_name(&self) -> Result<String, StreamError>;
}

// impl<S> Streamable for Box<S>
// where S: Streamable
// { }

pub mod plugins;
pub mod utils;
