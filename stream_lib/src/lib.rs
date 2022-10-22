//! This is a small tool to download streams
//! It currently supports chunked streams and HLS.

mod error;
pub mod hls;
mod download_stream;
//pub mod stream;

pub use crate::error::Error;
//pub use crate::stream::{Stream, StreamType};
pub use crate::download_stream::{DownloadStream, Event};

use crate::hls::HlsDownloader;
use reqwest::{Client, Request};

pub fn download_hls(http: Client, request: Request) -> DownloadStream {
    HlsDownloader::new(request, http).download()
}

pub fn download_hls_named(http: Client, request: Request, name: String) -> DownloadStream {
    HlsDownloader::new_named(request, http, name).download()
}
