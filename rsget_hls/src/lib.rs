extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate tokio;
//extern crate rsget_lib;
#[macro_use]
extern crate log;
extern crate http;


use hyper::Error as HyperError;
use std::io::Error as IoError;
use http::uri::InvalidUri as IError;

use http::Uri;
use http::Request;

use futures::{future, Future, Stream};
use std::fs::File;
use std::io::{self, Write};

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::error::Error as StdError;
use std::string::FromUtf8Error;

use tokio::prelude::future::ok;
//use rsget_lib::utils::error::StreamError;

type HttpsClient = hyper::Client<hyper_tls::HttpsConnector<hyper::client::HttpConnector>>;

macro_rules! ftry {
    ($code: expr) => {
        match $code {
            Ok(v) => v,
            Err(why) => return ::futures::future::err(From::from(why)),
        }
    };
}

#[derive(Debug)]
pub enum MyError {
    Hyper(HyperError),
    Io(IoError),
    InvalidUri(IError),
    Utf8(FromUtf8Error),
}

impl Display for MyError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        f.write_str(self.description())
    }
}

impl StdError for MyError {
    fn description(&self) -> &str {
        match *self {
            MyError::Hyper(ref inner) => inner.description(),
            MyError::Io(ref inner) => inner.description(),
            MyError::InvalidUri(ref inner) => inner.description(),
            MyError::Utf8(ref inner) => inner.description(),
        }
    }
}

impl From<HyperError> for MyError {
    fn from(err: HyperError) -> Self {
        MyError::Hyper(err)
    }
}

impl From<IoError> for MyError {
    fn from(err: IoError) -> Self {
        MyError::Io(err)
    }
}

impl From<IError> for MyError {
    fn from(err: IError) -> Self {
        MyError::InvalidUri(err)
    }
}

impl From<FromUtf8Error> for MyError {
    fn from(err: FromUtf8Error) -> Self {
        MyError::Utf8(err)
    }
}

pub fn download_to_string(client: HttpsClient, req: Request<hyper::Body>) -> impl Future<Item = String, Error = MyError> {
    //info!("Getting: {}", &uri);
    let f = client.request(req)
        .map_err(|e| MyError::from(e))
        .and_then(|resp| {
            debug!("Status: {}", resp.status());
            debug!("Headers:\n{:#?}", resp.headers());
            resp.into_body().concat2().map_err(|e| MyError::from(e)).map(|chunk| {
                let v = chunk.to_vec();
                String::from_utf8_lossy(&v).to_string()
            })
        });
    f
}

pub fn download_to_file(client: HttpsClient, req: Request<hyper::Body>, path: &str, spin: bool) -> impl Future<Item = (), Error = MyError> {
    let mut file = File::create(path).unwrap();
    
    client
        .request(req)
        .map_err(|e| MyError::from(e))
        .and_then(|res| {
            res.into_body()
                .map_err(|e| MyError::from(e))
                .for_each(move |chunk| {
                    info!("CHUNK!");
                    file
                        .write_all(&chunk)
                        .map_err(|e| MyError::from(e))
            })
        }).map_err(|e| MyError::from(e))
        .map(|_| ())
}

pub fn download_to_file2(client: HttpsClient, req: Request<hyper::Body>, path: &str, spin: bool) -> impl Future<Item = (), Error = MyError> {

    
    client
    // Fetch the url...
        .request(req)
    // And then, if we get a response back...
        .and_then(|res| {
            println!("Response: {}", res.status());
            println!("Headers: {:#?}", res.headers());
            let mut file = File::create("./test.test.test").unwrap();
            // The body is a stream, and for_each returns a new Future
            // when the stream is finished, and calls the closure on
            // each chunk of the body...
            res.into_body().for_each(move |chunk| {
                file
                    .write_all(&chunk)
                    .map_err(|e| panic!("example expects stdout is open, error={}", e))
            })
        })
    // If all good, just tell the user...
        .map(|_| {
            println!("\n\nDone.");
        })
    // If there was an error, let the user know...
        .map_err(|err| {
            panic!("Error {}", err);
        })
}
