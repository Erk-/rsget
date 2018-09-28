use utils::error::StreamError;
use utils::error::RsgetError;

use std::fs::File;
use std::io::Write;
//use std::io::BufWriter;
use std::process::Command;

use futures::{Stream, Future};

use tokio::runtime::Runtime;

use hls_m3u8::MediaPlaylist;

use indicatif::ProgressBar;

use serde::de::DeserializeOwned;

use reqwest;
use reqwest::Client as RClient;

use std::fs::create_dir;
use std::collections::HashSet;
use std::{thread, time};
//use std::fmt;

#[derive(Debug, Clone)]
pub struct DownloadClient {
    rclient: RClient,
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

        let mut bufw = BufWriter::with_capacity(131072, file);

        let spinner = ProgressBar::new(u64::MAX);
        spinner.set_style(ProgressStyle::default_bar()
                          .template("{spinner:.green} [{elapsed_precise}] Streamed {bytes}")
                          .tick_chars("⠁⠁⠉⠙⠚⠒⠂⠂⠒⠲⠴⠤⠄⠄⠤⠠⠠⠤⠦⠖⠒⠐⠐⠒⠓⠋⠉⠈⠈ "));

        let request = self.rclient.get(url);
        let mut source = DownloadProgress {
            progress_bar: spinner,
            inner: request.send()?,
        };

        let _ = copy(&mut source, &mut bufw)?;
        Ok(())
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
    
    pub fn download_to_file_future(&self, url: &str, file: File) -> Result<impl Future<Item = (), Error = StreamError>, StreamError> {
        use std::io::BufWriter;
        let mut fileb = BufWriter::new(file);
        let mut rt: Runtime = Runtime::new()?;
        use reqwest::async::Client as AsyncClient;
        let aclient = AsyncClient::new();
        let req1 = aclient.get(url);
        let resp_future = req1.send();
        let resp = rt.block_on(resp_future)?;
        info!("resp: {:#?}", &resp);
        let future = resp.into_body().map_err(StreamError::from).for_each(move |chunk| {
            fileb.write_all(&chunk)
                .map_err(StreamError::from)
        });
        Ok(future.map_err(StreamError::from))
    }

    pub fn hls_download(&self, master: &str, url: &str, folder: &str) -> Result<(), StreamError> {
        info!("Uses HLS download");
        let mut srt = Runtime::new()?;
        let mut links: HashSet<String> = HashSet::new();
        let mut counter = 0;
        let _ = create_dir(&folder);
        loop {
            let m3u8_str = self.download_to_string(self.make_request(&url, None)?)?;
            trace!("M3U8: {}", &m3u8_str);
            let m3u8 = m3u8_str.parse::<MediaPlaylist>()?;
            let m3u8_iterator = m3u8.segments().iter().map(|e| String::from(e.uri().trim()));
            for e in m3u8_iterator {
                if links.insert(e.clone()) {
                    debug!("Added: {:?}", &e);
                    let path_formatted = format!("{}/{}", &folder, &e.clone());
                    let url_formatted = format!("{}{}", &master, &e.clone());
                    trace!("Downloads {} to {}", &url_formatted, &path_formatted);
                    let mut file = File::create(path_formatted)?;
                    trace!("Before work");
                    let work = self.download_to_file_future(&url_formatted, file,)?
                        .map(|_| ())
                        .map_err(|_| ());
                    trace!("Adding work ({}) to the executor", counter);
                    srt.spawn(work);
                    counter += 1;
                }
            }
            thread::sleep(time::Duration::from_secs(5));
        }
    }
}

