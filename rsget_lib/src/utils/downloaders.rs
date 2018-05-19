use utils::error::StreamError;
use utils::error::RsgetError;

use std::fs::File;
use std::io::Write;
use std::process::Command;

use futures::{Future, Stream};
use hyper;
use tokio_core::reactor::Core;
use hyper::header::Location;
use reqwest;
use hls_m3u8::MediaPlaylist;

use indicatif::ProgressBar;

fn get_redirect_url(core: &mut Core, url: String) -> Result<String, StreamError> {
    let client = hyper::Client::new(&core.handle());

    let uri = url.parse()?;

    let work = client.get(uri);
    let res = match core.run(work) {
        Ok(r) => r,
        Err(why) => return Err(StreamError::Hyper(why)),
    };

    match res.headers().get::<Location>() {
        Some(loc) => Ok(loc.parse::<String>().unwrap()),
        None => Ok(url),
    }
}

pub fn flv_download(core: &mut Core, url: String, path: String) -> Result<(), StreamError> {
    let real_url = get_redirect_url(core, url)?;

    let client = hyper::Client::new(&core.handle());

    let mut file = File::create(&path)?;

    let uri = real_url.parse()?;
    let mut size: f64 = 0.0;
    let spinner = ProgressBar::new_spinner();
    let work = client.get(uri).and_then(|res| {
        res.body().for_each(|chunk| {
            spinner.tick();
            size = size + (chunk.len() as f64);
            spinner.set_message(&format!("Size: {:.2} MB", size / 1000.0 / 1000.0));
            file.write_all(&chunk).map_err(From::from)
        })
    });
    match core.run(work) {
        Ok(_) => Ok(()),
        Err(why) => Err(StreamError::Hyper(why)),
    }
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

