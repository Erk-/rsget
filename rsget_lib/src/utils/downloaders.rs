use utils::error::StreamError;
use utils::error::RsgetError;

use std::fs::File;
use std::io::Write;
//use std::io::BufWriter;
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

    
    pub fn hls_download(&self,
                        user_url: Option<&str>,
                        aid: Option<String>,
                        murl: String,
                        file: &File
    ) -> Result<(), StreamError> {
        use std::collections::VecDeque;
        use std::sync::Arc;
        use std::time::Duration;
        use parking_lot::Mutex;
        use std::thread;
        use std::collections::HashSet;
        use url::Url;
        use regex::Regex;
        use reqwest::header::REFERER;
        use hls_m3u8::MediaPlaylistOptions;
        use reqwest::header::USER_AGENT;
    
        #[derive(Clone)]
        enum Hls {
            Url(String),
            StreamOver,
        }
        
        let to_work = Arc::new(Mutex::new(VecDeque::<Hls>::new()));
        let other_work = to_work.clone();
        thread::spawn(move || {
            let inner_client = DownloadClient::new().unwrap();
            let mut links: HashSet<String> = HashSet::new();
            let mut counter = 0;
            let mut parsed_url = Url::parse(&murl).expect("[HLS] could not parse url");
            let master = parsed_url.join(".").expect("[HLS] Joining failed");
            let re_preloading = Regex::new(".*preloading.*").expect("[HLS] regex");
            if let Some(a) = aid {
                parsed_url.set_query(Some(&format!("aid={}",a)));
            }
            let url = parsed_url.into_string();
            loop {
                trace!("[HLS] First loop");
                if counter > 12 {
                    let to_add = &mut to_work.lock();
                    to_add.push_back(Hls::StreamOver);
                    break;
                }
                trace!("[HLS] Tries to get: {}", url);
                let req = match inner_client.make_request(&url, None) {
                    Ok(u) => u,
                    Err(e) => {
                        trace!("[HLS] breaks!!! ({})", e);
                        break;
                    },
                };
                trace!("[HLS] Begins download");
                let m3u8_str = match inner_client.download_to_string(req) {
                    Ok(s) => {
                        trace!("[HLS] M3U8:\n{}", s);
                        if s.is_empty() { continue; }
                        s
                    },
                    Err(e) => {
                        warn!("[HLS] Download failed! ({})", e);
                        counter += 1;
                        continue;
                    },
                };
                warn!("[HLS] M3U8: {}", &m3u8_str);
                let m3u8 = match MediaPlaylistOptions::new()
                    .allowable_excess_segment_duration(Duration::from_secs(10))
                    .parse(&m3u8_str) {
                        Ok(p) => {
                            p
                        },
                        Err(e) => {
                            warn!("[HLS] Parsing failed!\n{}", e);
                            trace!("[HLS]\n{}", &m3u8_str);
                            counter += 1;
                            continue;
                        },
                    };
                let target_duration = m3u8.target_duration_tag().duration();
                let m3u8_iterator = m3u8.segments().iter().map(|e| String::from(e.uri().trim()));
                for e in m3u8_iterator {
                    trace!("[HLS] Tries to inserts: {}", e);
                    if links.insert(e.clone()) {
                        counter = 0;
                        let url_formatted = format!("{}{}", &master, &e.clone());
                        let to_add = &mut to_work.lock();
                        if !re_preloading.is_match(&e) {
                            info!("[HLS] Adds {}!", url_formatted);
                            to_add.push_back(Hls::Url(url_formatted));
                        }
                    }
                }
                warn!("[HLS] Sleeps for {:#?}", target_duration);
                thread::sleep(target_duration);
                counter += 1;
            }
        });

        let mut size = 0;
        loop {
            trace!("[HLS] Second loop");
            let to_download = other_work.lock().pop_front().clone();
            match to_download {
                Some(Hls::Url(u)) => {
                    let c_file = file.try_clone()?;
                    let req = match user_url {
                        Some(uurl) => {
                            self.rclient
                                .get(&u)
                                .header(REFERER, uurl)
                                .header(USER_AGENT, "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/67.0.3396.62 Safari/537.36")
                        },
                        None => {
                            self.rclient
                                .get(&u)
                                .header(USER_AGENT, "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/67.0.3396.62 Safari/537.36")
                        },
                    };
                    
                    trace!("[HLS] Downloads: {:#?}", req);
                    size = self.download_to_file_request(req, c_file, false)?;
                },
                Some(Hls::StreamOver) => break,
                None => {
                    trace!("[HLS] None to download!");
                    thread::sleep(time::Duration::from_secs(5));
                },
            }
        }
        println!("[HLS] Downloaded: {} bytes", size);
        Ok(())
    }
}

