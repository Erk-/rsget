use utils::error::StreamError;
use utils::error::RsgetError;

use std::fs::File;
use std::io::Write;
use std::io::BufWriter;
use std::process::Command;

use futures::{Stream, Future};

use tokio::runtime::current_thread::Runtime;
//use tokio::runtime::Runtime as MRuntime;

use hyper;
use hyper::header::LOCATION;

use http::Request;

use hls_m3u8::MediaPlaylist;
//use hls_m3u8::MasterPlaylist;

use indicatif::ProgressBar;

use serde::de::DeserializeOwned;
use serde::ser;
use serde_json;

use serde_urlencoded;

//use tokio;

use std::fs::create_dir;
use std::collections::HashSet;
use std::{thread, time};

use HttpsClient;

#[derive(Debug, Clone)]
pub struct DownloadClient {
    client: HttpsClient,
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
    pub fn new(client: HttpsClient) -> Self {
        DownloadClient {
            client
        }
    }
    
    pub fn download_to_string(&self, req: Request<hyper::Body>) -> impl Future<Item = String, Error = StreamError> {
        self.client.request(req)
            .map_err(StreamError::from)
            .and_then(|resp| {
                debug!("Status: {}", resp.status());
                debug!("Headers:\n{:#?}", resp.headers());
                resp.into_body()
                    .concat2()
                    .map_err(StreamError::from)
                    .map(|chunk| {
                        let v = chunk.to_vec();
                        String::from_utf8_lossy(&v).to_string()
                    })
            })
    }

    pub fn get_redirection(&self, req: Request<hyper::Body>) -> hyper::client::ResponseFuture {
        trace!("Enters `get_redirection`");
        let mut runtime = Runtime::new().unwrap();
        let ouri = req.uri().clone();
        let work = self.client.request(req)
            .map(|r|
                 if r.status().is_redirection() {
                     Some(r.headers()[LOCATION]
                          .to_str()
                          .unwrap()
                          .parse()
                          .unwrap())
                 } else {
                     None
                 }
            );

        let resp: Option<hyper::Uri> = runtime.block_on(work).unwrap();
        
        match resp {
            Some(uri) => {
                let new_req = self.make_request(&uri.to_string(), None);
                self.client.request(new_req.unwrap())
            },
            None => {
                let new_req = self.make_request(&ouri.to_string(), None);
                self.client.request(new_req.unwrap())
            },
        }
    }

    pub fn download_to_file(&self, req: Request<hyper::Body>, file: File, spin: bool) -> impl Future<Item = (), Error = StreamError> {
        //let mut file = File::create(path).unwrap();
        let mut filew = BufWriter::new(file);
        let resp = self.get_redirection(req);
        resp
            .map_err(StreamError::from)
            .and_then(move |res| {
                debug!("dtf Status: {}", res.status());
                debug!("dtf Headers:\n{:#?}", res.headers());
                let mut size: f64 = 0.0;
                let spinner = ProgressBar::new_spinner();
                res.into_body().map_err(|e| StreamError::from(e)).for_each(move |chunk| {
                    if spin {
                        spinner.tick();
                        size += chunk.len() as f64;
                        spinner.set_message(&format!("Size: {:.2} MB", size / 1000.0 / 1000.0));
                    }
                    filew.write_all(&chunk)
                        .map_err(StreamError::from)
                })
            }).map(|_| ())
    }

    pub fn download_and_de<T: DeserializeOwned>(&self, req: Request<hyper::Body>) -> impl Future<Item = Result<T,StreamError>, Error = StreamError> {
        self.client.request(req)
            .map_err(StreamError::from)
            .and_then(|resp| {
                debug!("Status: {}", resp.status());
                debug!("Headers:\n{:#?}", resp.headers());
                resp.into_body().concat2().map_err(StreamError::from).map(|chunk| {
                    let v = chunk.to_vec();
                    debug!("To be deseriazed:\n{:?}", String::from_utf8(v.clone()).unwrap());
                    let ds: Result<T,StreamError> = serde_json::from_slice(&v).map_err(StreamError::from);
                    ds
                })
            })
    }

    pub fn make_request(&self, uri: &str, headers: Option<(&str, &str)>) -> Result<Request<hyper::Body>, StreamError> {
        let req = match headers {
            Some(a) => {
                Request::builder()
                    .uri(uri)
                    .header(a.0,a.1)
                    .body(Default::default())
            },
            None => {
                Request::builder()
                    .uri(uri)
                    .body(Default::default())
            }
        };
        req.map_err(StreamError::from)
    }

    pub fn make_post_request(&self, uri: &str, headers: Option<(&str, &str)>) -> Result<Request<hyper::Body>, StreamError> {
        let req = match headers {
            Some(a) => {
                Request::post(uri)
                    .header(a.0,a.1)
                    .body(Default::default())?
            },
            None => {
                Request::post(uri)
                    .body(Default::default())?
            }
        };
        trace!("Request: {:?}", &req);
        trace!("Body: {:?}", req.body());
        Ok(req)
    }

    pub fn make_request_body<T>(&self, uri: &str, headers: Option<(&str, &str)>, body: T) -> Result<Request<hyper::Body>, StreamError>
    where T: ser::Serialize,
    {
        let body_string = serde_urlencoded::to_string(body)?;
        trace!("To be body: {}", &body_string);
        let new_body = hyper::Body::from(body_string);
        let req = match headers {
            Some(a) => {
                Request::post(uri)
                    .header(a.0,a.1)
                    .header("content-type", "application/x-www-form-encoded")
                    .header("User-Agent","Hyper/0.12.0")
                    .body(new_body)?
            },
            None => {
                Request::post(uri)
                    .header("content-type", "application/x-www-form-encoded")
                    .header("User-Agent","Hyper/0.12.0")
                    .body(new_body)?
            }
        };
        trace!("Request: {:?}", &req);
        Ok(req)
    }

    pub fn serialize_request<T>(&self, req: Request<T>) -> serde_json::Result<Request<Vec<u8>>>
    where T: ser::Serialize,
    {
        let (parts, body) = req.into_parts();
        let body = serde_json::to_vec(&body)?;
        Ok(Request::from_parts(parts, body))
    }

    pub fn download_to_file_no_redir(&self, req: Request<hyper::Body>, mut file: File, spin: bool) -> impl Future<Item = (), Error = StreamError> {
        trace!("Enters: `download_to_file_no_redir`");
        //let mut file = File::create(path).unwrap();
        let resp = self.client.request(req);
        resp
            .map_err(StreamError::from)
            .and_then(move |res| {
                debug!("dtf Status: {}", res.status());
                debug!("dtf Headers:\n{:#?}", res.headers());
                let mut size: f64 = 0.0;
                let spinner = ProgressBar::new_spinner();
                res.into_body().map_err(StreamError::from).for_each(move |chunk| {
                    if spin {
                        spinner.tick();
                        size += chunk.len() as f64;
                        spinner.set_message(&format!("Size: {:.2} MB", size / 1000.0 / 1000.0));
                    }
                    file.write_all(&chunk)
                        .map_err(StreamError::from)
                })
            }).map(|_| ())
    }

    pub fn hls_download(&self, master: String, url: String, folder: String) -> Result<(), StreamError> {
        info!("Uses HLS download");
        let mut srt = Runtime::new().unwrap();
        let mut links: HashSet<String> = HashSet::new();
        let mut counter = 0;
        let _ = create_dir(&folder);
        loop {
            let m3u8_str = srt.block_on(self.download_to_string(self.make_request(&url, None)?))?;
            trace!("M3U8: {}", &m3u8_str);
            let m3u8 = m3u8_str.parse::<MediaPlaylist>()?;
            let m3u8_iterator = m3u8.segments().iter().map(|e| String::from(e.uri().trim()));
            for e in m3u8_iterator {
                if links.insert(e.clone()) {
                    debug!("Added: {:?}", &e);
                    let path_formatted = format!("{}/{}.ts", &folder, counter);
                    let url_formatted = format!("{}{}", &master, &e.clone());
                    trace!("Downloads {} to {}", &url_formatted, &path_formatted);
                    let ts_req = self.make_request(&url_formatted, None)?;
                    let mut file = File::create(path_formatted)?;
                    trace!("Before work");
                    let work = self.download_to_file_no_redir(ts_req,
                                                         file,
                                                         false
                    ).map(|_| ()).map_err(|_| ());
                    trace!("Adding work ({}) to the executor", counter);
                    srt.spawn(work);
                    counter += 1;
                }
            }
            thread::sleep(time::Duration::from_secs(5));
        }
    }
}
