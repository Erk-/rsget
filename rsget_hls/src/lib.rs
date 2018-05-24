extern crate futures;
extern crate hyper;
extern crate hyper_tls;
extern crate tokio;
//extern crate rsget_lib;
#[macro_use]
extern crate log;


use hyper::Error as HyperError;
use std::io::Error as IoError;

use futures::{future, Future, Stream};
use std::fs::File;
use std::io::{self, Write};

use std::fmt::{Display, Formatter, Result as FmtResult};
use std::error::Error as StdError;


//use rsget_lib::utils::error::StreamError;

#[derive(Debug)]
enum MyError {
    Hyper(HyperError),
    Io(IoError),
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


pub fn download_to_file(uri: &str, path: &str) -> impl Future<Item = (), Error = MyError> {
    let https = hyper_tls::HttpsConnector::new(4).unwrap();
    let client = hyper::Client::builder()
        .build::<_, hyper::Body>(https);
    let mut file = File::create(path).unwrap();
    client
        .get(uri.parse().unwrap())
        .and_then(move |res| {
            //println!("Status: {}", res.status());
            //println!("Headers:\n{:#?}", res.headers());
            res.into_body().map_err(|e| MyError::from(e)).for_each(move |chunk| {
                file
                    .write_all(&chunk)
                    .map_err(|e| MyError::from(e))
            })
        }).map_err(|e| MyError::from(e))
        .map(|_| ())
}

/*
pub fn downloader(uri: &str, path: &str) -> impl Future<Item = (), Error = ()> {
    test_download(uri, path).then(|_| Ok(()))
}
*/

/*
pub fn test_download() -> () {
    tokio::run(future::lazy(|| {
        let https = hyper_tls::HttpsConnector::new(4).unwrap();
        let client = hyper::Client::builder()
            .build::<_, hyper::Body>(https);
        let mut file = File::create("./test.test").unwrap();
        client
            .get("https://hyper.rs".parse().unwrap())
            .and_then(|res| {
                println!("Status: {}", res.status());
                println!("Headers:\n{:#?}", res.headers());
                res.into_body().for_each(move |chunk| {
                    file
                        .write_all(&chunk)
                        .map_err(|e| panic!("example expects stdout to work: {}", e))
                })
            })
            .map_err(|e| println!("request error: {}", e))
    }));
    ()
}
*/
/*
extern crate hls_m3u8;
extern crate futures;
extern crate hyper;
extern crate tokio_core;
extern crate regex;
#[macro_use] extern crate log;
extern crate hyper_tls;
extern crate reqwest;

use std::io::{self, Write};
use std::io::Error as IoError;
use std::{thread, time};
use futures::{Stream, Sink, Future};
use futures::sync::mpsc;
use hyper::Client;
use hyper_tls::HttpsConnector;
use tokio_core::reactor::Core;
use hls_m3u8::MediaPlaylist;
use hyper::Uri;
use regex::Regex;
use std::fs::File;

fn download_future(client: hyper::Client<HttpsConnector>, path: String, uri: Uri) -> FutureResult<Item=usize, Error=std::io::Error>  {
    let file = File::create(&path).unwrap();
    client.get(uri).and_then(|res| {
        res.body().for_each(|chunk| {
            file.write_all(&chunk).map_err(From::from)
        })
    });
}
*/

/*
fn hls_get_file(r: reqwest::Client, uri: &str) -> MediaPlaylist {
    let text = r.get(uri).send().unwrap().text().unwrap();
    text.parse::<MediaPlaylist>().unwrap()
}

fn reg_to_n(r: Regex, s: String) -> usize {
    (r.captures(&s).unwrap()[1]).parse::<usize>().unwrap()
}

pub fn hls_download(url: String, path: String) -> Option<()> {
    let mut core = match Core::new() {
        Ok(s) => s,
        Err(_) => {
            debug!("EHH");
            return None
        },
    };

    let re_baseurl: Regex = Regex::new(r"(.+)(?:/[^/].m3u8.*)").unwrap();
    let re_index: Regex = Regex::new(r"(?:.+_)([0-9]+)\.TS").unwrap();

    let baseurl = &(re_baseurl.captures(&url).unwrap())[1];
    let client = ::hyper::Client::configure()
        .connector(::hyper_tls::HttpsConnector::new(4, &core.handle()).unwrap())
        .build(&core.handle());
    let rclient = reqwest::Client::new();
    let uri = url.parse::<Uri>().unwrap();
    let ihls = hls_get_file(rclient, &url);
    let mut cindex = 0;
    let remote = core.remote();
    let (tx, rx) = mpsc::channel(1);

    thread::spawn(move || {
        loop {
            let tx = tx.clone();

            // INSERT WORK HERE - the work should be modeled as having a _future_ result.
            let delay = time::Duration::from_secs(1);
            thread::sleep(delay);
            let tmp_pl = hls_get_file(rclient, &url);
            let mut i = 0;
            while(i < 5 && cindex < reg_to_n(re_index, tmp_pl.segments()[0].uri().to_string())){
                i = i + 1;
            }
            let file = File::create(format!("test_{}.TS", i)).unwrap();
            let f = client.get(format!("{}{}",
                                       baseurl,
                                       tmp_pl.segments()[i].uri()).parse::<Uri>().unwrap()).and_then(|res| {
                res.body().for_each(|chunk| {
                    file.write_all(&chunk).map_err(From::from)
                })
            });
            remote.spawn(|_| {
                f.then(|res| {
                    tx
                        .send(res)
                        .then(|tx| {
                            match tx {
                                Ok(_tx) => {
                                    info!("Sink flushed");
                                    Ok(())
                                }
                                Err(e) => {
                                    error!("Sink failed! {:?}", e);
                                    Err(())
                                }
                            }
                        }) 
                })
            });
        }
    });

    let f2 = rx.for_each(|res| {
        match res {
            Ok(_) => println!("NICE!!"),
            Err(_) => println!("LESS NICE!!"),
        }
        Ok(())
    });

    
    match core.run(f2) {
        Ok(_) => {
            println!("øhh");
            Some(())
        },
        Err(e) => {
            None
        },
    }
}
*/
/*
pub fn flv_download(core: &mut Core, url: String, path: String) -> Result<(), StreamError> {
    let real_url = get_redirect_url(core, url)?;

    let client = hyper::Client::new(&core.handle());

    let mut file = File::create(&path)?;

    let uri = real_url.parse()?;
    let mut size: f64 = 0.0;
    let spinner = ProgressBar::new_spinner();
    let work = client.get(uri).and_then(|res| {
        res.body().for_each(|chunk| {
            file.write_all(&chunk).map_err(From::from)
        })
    });
    match core.run(work) {
        Ok(_) => Ok(()),
        Err(why) => Err(StreamError::Hyper(why)),
    }
}

*/
