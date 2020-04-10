//! This is a small tool to download streams
//! It currently supports chunked streams and HLS.

#[macro_use]
extern crate log;

mod error;
pub mod hls;
pub mod named_hls;
pub mod stream;

pub use crate::error::Error;
pub use crate::hls::HlsDownloader;
pub use crate::stream::{Stream, StreamType};
