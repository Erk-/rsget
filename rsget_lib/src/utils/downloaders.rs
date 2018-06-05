use utils::error::StreamError;
use utils::error::RsgetError;

use std::fs::File;
use std::io::Write;
use std::process::Command;

use futures::{Stream, Future};

use tokio::runtime::current_thread::Runtime;

use hyper;
use hyper::header::LOCATION;

use http::Request;

//use hls_m3u8::MediaPlaylist;

use indicatif::ProgressBar;

use serde::de::DeserializeOwned;
use serde::ser;
use serde_json;

use HttpsClient;

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

pub fn download_to_string(client: &HttpsClient, req: Request<hyper::Body>) -> impl Future<Item = String, Error = StreamError> {
    client.request(req)
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

pub fn get_redirection(client: &HttpsClient, req: Request<hyper::Body>) -> hyper::client::ResponseFuture {
    let mut runtime = Runtime::new().unwrap();
    let ouri = req.uri().clone();
    let work = client.request(req)
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
            let new_req = make_request(&uri.to_string(), None);
            client.request(new_req.unwrap())
        },
        None => {
            let new_req = make_request(&ouri.to_string(), None);
            client.request(new_req.unwrap())
        },
    }
}

pub fn download_to_file(client: &HttpsClient, req: Request<hyper::Body>, mut file: File, spin: bool) -> impl Future<Item = (), Error = StreamError> {
    //let mut file = File::create(path).unwrap();
    let resp = get_redirection(client,req);
    resp
        .map_err(|e| StreamError::from(e))
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
                file.write_all(&chunk)
                    .map_err(StreamError::from)
            })
        }).map(|_| ())
}

pub fn download_and_de<T: DeserializeOwned>(client: &HttpsClient, req: Request<hyper::Body>) -> impl Future<Item = Result<T,StreamError>, Error = StreamError> {
    client.request(req)
        .map_err(StreamError::from)
        .and_then(|resp| {
            debug!("Status: {}", resp.status());
            debug!("Headers:\n{:#?}", resp.headers());
            resp.into_body().concat2().map_err(StreamError::from).map(|chunk| {
                let v = chunk.to_vec();
                let ds: Result<T,StreamError> = serde_json::from_slice(&v).map_err(StreamError::from);
                ds
            })
        })
}

pub fn make_request(uri: &str, headers: Option<(&str, &str)>) -> Result<Request<hyper::Body>, StreamError> {
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

pub fn make_request_2<T>(uri: &str, headers: Option<(&str, &str)>, body: T) -> Result<Request<T>, StreamError> {
    let req = match headers {
        Some(a) => {
            Request::builder()
                .uri(uri)
                .header(a.0,a.1)
                .body(body)
        },
        None => {
            Request::builder()
                .uri(uri)
                .body(body)
        }
    };
    req.map_err(StreamError::from)
}

pub fn serialize_request<T>(req: Request<T>) -> serde_json::Result<Request<Vec<u8>>>
    where T: ser::Serialize,
{
    let (parts, body) = req.into_parts();
    let body = serde_json::to_vec(&body)?;
    Ok(Request::from_parts(parts, body))
}
