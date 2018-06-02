extern crate futures;
extern crate rsget_hls;
extern crate pretty_env_logger;
#[macro_use] extern crate log;
extern crate tokio;
extern crate hyper;
extern crate hyper_tls;
extern crate http;

use futures::{future, Future, Stream};
use tokio::runtime::current_thread::Runtime;
use tokio::executor::current_thread::CurrentThread;

use std::fs::File;

use http::Uri;
use http::Request;

use rsget_hls::*;
use rsget_hls::MyError;

fn main() {
    pretty_env_logger::init();
    
    let mut runtime = Runtime::new().unwrap();

    let https = hyper_tls::HttpsConnector::new(4).unwrap();
    let client = hyper::Client::builder()
        .build::<_, hyper::Body>(https);
    let req = make_request("https://www.mediacollege.com/video-gallery/testclips/barsandtone.flv", None);
    let fut = download_to_file2(client, req, "./test.flv", false).map_err(|_| ());
    tokio::run(fut);
}

fn make_request(uri: &str, headers: Option<(&str, &str)>) -> Request<hyper::Body> {
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

