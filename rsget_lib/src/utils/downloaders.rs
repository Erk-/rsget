use utils::error::StreamError;
use utils::error::RsgetError;

use std::fs::File;
use std::io::Write;
use std::process::Command;

use futures::{Stream, Future};

use futures::Async::Ready;
use futures::Async::NotReady;

use tokio_fs::file::File as TokioFile;

use tokio_core::reactor::Core;

use hyper::body::Payload;
use hyper;
use hyper_tls;

use http::header::{self};//, HeaderName};
use http::Request;

//use hls_m3u8::MediaPlaylist;

//use indicatif::ProgressBar;

use serde::de::DeserializeOwned;
use serde_json;

type HttpsClient = hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>;

fn get_redirect_url(core: &mut Core, url: String) -> Result<String, StreamError> {
    let client = hyper::Client::new();
    let uri = url.parse()?;

    let work = client.get(uri);
    let res = match core.run(work) {
        Ok(r) => r,
        Err(why) => return Err(StreamError::Hyper(why)),
    };

    let headers = res.headers();

    if headers.contains_key(header::LOCATION) {
        Ok(String::from(headers[header::LOCATION].to_str()?))
    } else {
        Ok(url)
    }
    /*
    match res.headers().get(header::LOCATION) {
        Some(loc) => Ok(loc.parse::<String>().unwrap()),
        None => Ok(url),
    }
    */
}

pub fn flv_download(_core: &mut Core, _url: String, _path: String) -> Result<(), StreamError> {
    Err(StreamError::Rsget(RsgetError::new("NOT IMPLEMENTED!!")))
    /*
    let real_url = get_redirect_url(core, url)?;

    let client = hyper::Client::new();

    let mut file = File::create(&path)?;

    let uri = real_url.parse()?;
    let mut size: f64 = 0.0;
    let spinner = ProgressBar::new_spinner();
    let work = client
        .get(uri)
        .map_err(|e| StreamError::from(e))
        .and_then(|res| {
            res.body()
                .map_err(|e| StreamError::from(e))
                .for_each(|chunk| {
                    spinner.tick();
                    size = size + (chunk.len() as f64);
                    spinner.set_message(&format!("Size: {:.2} MB", size / 1000.0 / 1000.0));
                    file
                        .write_all(&chunk)
                        .map_err(|e| StreamError::from(e))
                })
        });
    core.run(work)
        */
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

pub fn download_to_string(client: HttpsClient, req: Request<hyper::Body>) -> impl Future<Item = String, Error = StreamError> {
    let f = client.request(req)
        .map_err(|e| StreamError::from(e))
        .and_then(|resp| {
            debug!("Status: {}", resp.status());
            debug!("Headers:\n{:#?}", resp.headers());
            resp.into_body().concat2().map_err(|e| StreamError::from(e)).map(|chunk| {
                let v = chunk.to_vec();
                String::from_utf8_lossy(&v).to_string()
            })
        });
    f
}

/*
client
    .get(uri)
    .map_err(|e| StreamError::from(e))
    .and_then(|res| {
        res.body()
            .map_err(|e| StreamError::from(e))
            .for_each(|chunk| {
                spinner.tick();
                size = size + (chunk.len() as f64);
                spinner.set_message(&format!("Size: {:.2} MB", size / 1000.0 / 1000.0));
                file
                    .write_all(&chunk)
                    .map_err(|e| StreamError::from(e))
            })
    });
*/

pub fn download_to_file(client: HttpsClient, req: Request<hyper::Body>, _path: String, _spin: bool) -> impl Future<Item = (), Error = StreamError> {
    let mut file = File::create("./test.flv").unwrap();
    client
        .request(req)
        .map_err(|e| StreamError::from(e))
        .map(|res| {
            res.into_body().map(move |chunk| {
                    file.write_all(&chunk)
                        .map_err(|e| StreamError::from(e))
                })
        }).map(|_| ())
}

pub fn download_and_de<T: DeserializeOwned>(client: HttpsClient, req: Request<hyper::Body>) -> impl Future<Item = Result<T,StreamError>, Error = StreamError> {
    let f = client.request(req)
        .map_err(|e| StreamError::from(e))
        .and_then(|resp| {
            debug!("Status: {}", resp.status());
            debug!("Headers:\n{:#?}", resp.headers());
            resp.into_body().concat2().map_err(|e| StreamError::from(e)).map(|chunk| {
                let v = chunk.to_vec();
                let ds: Result<T,StreamError> = serde_json::from_slice(&v).map_err(|e| StreamError::from(e));
                ds
            })
        });
    f
}

pub fn make_request(uri: &str, headers: Option<(&str, &str)>) -> Request<hyper::Body> {
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
    req.unwrap()
}
