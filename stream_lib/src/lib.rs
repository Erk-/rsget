//! This is a small tool to download streams
//! It currently supports chunked streams and HLS.

mod download_stream;
mod error;
mod hls;

use std::time::Duration;

pub use crate::download_stream::{DownloadStream, Event};
pub use crate::error::Error;

use crate::hls::HlsDownloader;
use hls::download_to_file;
use reqwest::{Client, Request};

pub fn download_hls(
    http: Client,
    request: Request,
    filter: Option<fn(&str) -> bool>,
) -> DownloadStream {
    HlsDownloader::new(request, http, filter).download()
}

pub fn download_hls_named(
    http: Client,
    request: Request,
    name: String,
    filter: Option<fn(&str) -> bool>,
) -> DownloadStream {
    HlsDownloader::new_named(request, http, name, filter).download()
}

pub fn download_chunked(http: Client, request: Request) -> DownloadStream {
    let (dl, tx) = DownloadStream::new();

    tokio::spawn(download_to_file(
        http,
        request,
        tx,
        None,
        Some(Duration::from_secs(60)),
    ));
    dl
}
