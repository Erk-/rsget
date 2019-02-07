//! This is a small tool to download streams
//! It currently supports chunked streams and HLS.

#[macro_use]
extern crate log;

mod error;
mod stream;

pub use error::Error;
pub use stream::{Stream, StreamType};
