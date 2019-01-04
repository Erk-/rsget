use utils::error::StreamError;
use utils::error::RsgetError;

use std::fs::File;
use std::io::Write;
use std::process::Command;

use futures::{Stream, Future};

use tokio::runtime::Runtime;

use indicatif::ProgressBar;

use serde::de::DeserializeOwned;

use reqwest;
use reqwest::Client as RClient;

use std::time;

#[derive(Debug, Clone)]
pub struct DownloadClient {
    pub rclient: RClient,
}

pub fn ffmpeg_download(url: String, path: String) -> Result<(), StreamError> {
    let comm = Command::new("ffmpeg")
        .arg("-i")
        .arg(url)
        .arg("-c")
        .arg("copy")
        .arg(path)
        .status()
        .expect("ffmpeg failed to start");
    match comm.code() {
        Some(c) => {
            info!("Ffmpeg returned: {}", c);
            Ok(())
        },
        None => {
            info!("Err: Ffmpeg failed");
            Err(StreamError::Rsget(RsgetError::new("Ffmpeg failed")))
        },
    }
}

impl DownloadClient {
    pub fn new() -> Result<Self, StreamError> {
        Ok(DownloadClient {
            rclient: RClient::new(),
        })
    }

    pub fn download_to_string(&self, req: reqwest::Request) -> Result<String, StreamError> {
        let c = &self.rclient;
        let mut res = c.execute(req)?;
        res.text().map_err(StreamError::from)
    }
    
    pub fn download_to_file(&self, url: &str, file: File, _spin: bool) -> Result<(), StreamError>{
        info!("Downloads file: {}", url);
        use std::io::Read;
        use std::io::copy;
        use std::io::Result as IoResult;
        use std::io::BufWriter;
        use indicatif::ProgressStyle;
        use std::u64;

        struct DownloadProgress<R> {
            inner: R,
            progress_bar: ProgressBar,
        }

        impl<R: Read> Read for DownloadProgress<R> {
            fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
                self.inner.read(buf).map(|n| {
                    self.progress_bar.inc(n as u64);
                    n
                })
            }
        }

        let mut bufw = BufWriter::with_capacity(131_072, file);

        let size = if _spin {
            reqwest::get(url)?.headers()
                .get(reqwest::header::CONTENT_LENGTH)
                .and_then(|ct_len| ct_len.to_str().ok())
                .and_then(|ct_len| ct_len.parse().ok())
                .unwrap_or(0)
        } else { u64::MAX };

        let spinner = ProgressBar::new(size);
        if _spin {
            spinner.set_style(ProgressStyle::default_bar()
                              .template("{spinner:.green} [{elapsed_precise}]  [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                              .tick_chars("⠁⠁⠉⠙⠚⠒⠂⠂⠒⠲⠴⠤⠄⠄⠤⠠⠠⠤⠦⠖⠒⠐⠐⠒⠓⠋⠉⠈⠈ "));
        } else {
            spinner.set_style(ProgressStyle::default_bar()
                              .template("{spinner:.green} [{elapsed_precise}] Streamed {bytes}")
                              .tick_chars("⠁⠁⠉⠙⠚⠒⠂⠂⠒⠲⠴⠤⠄⠄⠤⠠⠠⠤⠦⠖⠒⠐⠐⠒⠓⠋⠉⠈⠈ "));
        }
        let request = self.rclient.get(url);
        let mut source = DownloadProgress {
            progress_bar: spinner,
            inner: request.send()?,
        };

        let _ = copy(&mut source, &mut bufw)?;
        Ok(())
    }

    pub fn download_to_file_request(&self, request: reqwest::RequestBuilder, file: File, _spin: bool) -> Result<u64, StreamError>{
        use std::io::Read;
        use std::io::copy;
        use std::io::Result as IoResult;
        use std::io::BufWriter;
        use indicatif::ProgressStyle;
        use std::u64;

        struct DownloadProgress<R> {
            inner: R,
            progress_bar: ProgressBar,
        }


        impl<R: Read> Read for DownloadProgress<R> {
            fn read(&mut self, buf: &mut [u8]) -> IoResult<usize> {
                self.inner.read(buf).map(|n| {
                    self.progress_bar.inc(n as u64);
                    n
                })
            }
        }

        let mut bufw = BufWriter::with_capacity(131_072, file);

        let spinner = ProgressBar::new(u64::MAX);
        spinner.set_style(ProgressStyle::default_bar()
                          .template("{spinner:.green} [{elapsed_precise}] Streamed {bytes}")
                          .tick_chars("⠁⠁⠉⠙⠚⠒⠂⠂⠒⠲⠴⠤⠄⠄⠤⠠⠠⠤⠦⠖⠒⠐⠐⠒⠓⠋⠉⠈⠈ "));

        let mut source = DownloadProgress {
            progress_bar: spinner,
            inner: request.send()?,
        };

        let n = copy(&mut source, &mut bufw)?;
        Ok(n)
    }

    
    pub fn download_and_de<T: DeserializeOwned>(&self, req: reqwest::Request) -> Result<T,StreamError> {
        let c = &self.rclient;
        let mut res = c.execute(req)?;
        let json: T = res.json()?;
        Ok(json)
    }

    pub fn make_request(&self, uri: &str, headers: Option<(&str, &str)>) -> Result<reqwest::Request, StreamError> {
        let c = &self.rclient;
        match headers {
            Some(a) => {
                c.get(uri)
                 .header(a.0, a.1).build().map_err(StreamError::from)
            },
            None => {
                c.get(uri).build().map_err(StreamError::from)
            }
        }
    }
}
