/// Write buffer
pub const WRITE_SIZE: usize = 131_072;

/// HLS will try and look for new segments 12 times,
pub const HLS_MAX_RETRIES: usize = 12;

use std::time::Duration;

use hls_m3u8::MediaPlaylist;
use patricia_tree::PatriciaSet;

use reqwest::{Url, Client, Request};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::{warn, trace, info};

use crate::Error;


pub struct HlsWatch {
    tx: UnboundedSender<HlsQueue>,
    request: Request,
    http: Client,
    links: PatriciaSet,
    master_url: Url,
    timeout: Duration,
}

#[derive(Debug, Clone)]
pub enum HlsQueue {
    Url(Url),
    StreamOver,
}

impl HlsWatch {
    pub fn new(request: Request, http: Client) -> (Self, UnboundedReceiver<HlsQueue>) {
        let (tx, rx) = unbounded_channel();
        let master_url = request
            .url()
            .clone()
            .join(".")
            .expect("Could not join url with '.'.");
        (
            HlsWatch {
                tx,
                request,
                http,
                links: PatriciaSet::new(),
                master_url,
            },
            rx,
        )
    }

    async fn run(mut self) -> Result<(), Error> {
        info!("STARTING WATCH!");
        let mut fail_counter = 0;

        loop {
            if fail_counter > HLS_MAX_RETRIES {
                // There have either been errors or no new segments
                // for `HLS_MAX_RETRIES` times the segment duration given
                // in the m3u8 playlist file.
                if self.tx.send(HlsQueue::StreamOver).is_err() {
                    return Err(Error::TIO(std::io::Error::last_os_error()));
                };
                break;
            }

            // Use the same headers as the original request
            let req = match self.request.try_clone() {
                Some(mut r) => {
                    *r.timeout_mut() = Some(Duration::from_secs(10));
                    r
                }
                // If the body is not able to be cloned it will only clone the headers.
                None => {
                    warn!("[HLS] body not able to be cloned only clones headers.");
                    match self
                        .http
                        .get(self.request.url().clone())
                        .headers(self.request.headers().clone())
                        .timeout(Duration::from_secs(10))
                        .build()
                    {
                        Ok(br) => br,
                        Err(e) => {
                            warn!("[HLS] Request creation failed!\n{}", e);
                            fail_counter += 1;
                            continue;
                        }
                    }
                }
            };

            let res = match self.http.execute(req).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("[HLS] Playlist download failed!\n{}", e);
                    fail_counter += 1;
                    continue;
                }
            };

            let m3u8_string = match res.text().await {
                Ok(t) => t,
                Err(e) => {
                    warn!("[HLS] Playlist text failed!\n{}", e);
                    fail_counter += 1;
                    continue;
                }
            };

            let mut m3u8_parser = MediaPlaylist::builder();

            // Allow excess segment duration because a lot of video sites have
            // not very high quality m3u8 playlists, where the video segments,
            // may be longer than what the file specifies as max.
            m3u8_parser.allowable_excess_duration(Duration::from_secs(10));

            let m3u8 = match m3u8_parser.parse(&m3u8_string) {
                Ok(p) => p,
                Err(e) => {
                    warn!("[HLS] Parsing failed!\n{}", e);
                    trace!("[HLS]\n{}", &m3u8_string);
                    fail_counter += 1;
                    continue;
                }
            };

            // Get the target duration of a segment
            let target_duration = m3u8.target_duration;

            // Makes a iterator with the url parts from the playlist
            let m3u8_iterator = m3u8
                .segments
                .iter()
                .map(|(_, e)| String::from(e.uri().trim()));

            for e in m3u8_iterator {
                trace!("[HLS] Tries to inserts: {}", e);
                // Check if we have the segment in our set already
                if self.links.insert(e.clone()) {
                    // Reset the counter as we got a new segment.
                    fail_counter = 0;

                    // Construct a url from the master and the segment.
                    let url_formatted = if let Ok(u) = Url::parse(&e) {
                        u
                    } else {
                        Url::parse(&format!("{}{}", self.master_url.as_str(), &e)).expect(
                            "The m3u8 does not currently work with stream_lib, \
                                     please report the issue on the github repo, with an \
                                     example of the file if possible.",
                        )
                    };

                    // Check if the segment is a Afreeca preloading segment.
                    if !(e.contains("preloading")) {
                        info!("[HLS] Adds {}!", url_formatted);
                        // Add the segment to the queue.
                        if self.tx.send(HlsQueue::Url(url_formatted)).is_err() {
                            return Err(Error::TIO(std::io::Error::last_os_error()));
                        };
                    }
                }
            }
            warn!("[HLS] Sleeps for {:#?}", target_duration);
            // Sleeps for the target duration.
            tokio::time::sleep(target_duration).await;
            fail_counter += 1;
        }

        Ok(())
    }
}

