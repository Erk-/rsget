#![allow(clippy::new_ret_no_self)]
#![deny(rust_2018_idioms)]

#[macro_use]
extern crate log;
#[macro_use]
extern crate serde_derive;

use crate::utils::error::StreamError;
use crate::utils::error::StreamResult;

use std::boxed::Box;

use stream_lib::StreamType;

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
    async fn new(url: String) -> StreamResult<Box<Self>>
    where
        Self: Sized + Sync;
    /// Returns the title of the stream if possible
    async fn get_title(&self) -> StreamResult<String>;
    /// Returns the author of the stream if possible
    async fn get_author(&self) -> StreamResult<String>;
    /// Returns if the stream is online
    async fn is_online(&self) -> StreamResult<Status>;
    /// Gets the url of the stream
    async fn get_stream(&self) -> StreamResult<StreamType>;
    /// Returns what extension the stream should be
    async fn get_ext(&self) -> StreamResult<String>;
    /// Gets the default name of the stream
    async fn get_default_name(&self) -> StreamResult<String>;
}

// impl<S> Streamable for Box<S>
// where S: Streamable
// { }

pub mod plugins;
pub mod utils;
