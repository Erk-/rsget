//! This is a small tool to download streams
//! It currently supports chunked streams and HLS.

#[macro_use]
extern crate log;

mod error;
//mod stream;
pub mod hls;

pub use crate::error::Error;
pub use crate::hls::HlsDownloader;
//pub use crate::stream::{Stream, StreamType};
