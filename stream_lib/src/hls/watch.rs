use std::time::Duration;

use hls_m3u8::MediaPlaylist;
use patricia_tree::PatriciaSet;

use reqwest::{Client, Request, Url};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::{info, trace, warn};

use crate::{
    hls::{clone_request, HlsQueue, HLS_MAX_RETRIES},
    Error,
};

pub struct HlsWatch {
    tx: UnboundedSender<HlsQueue>,
    request: Request,
    http: Client,
    links: PatriciaSet,
    master_url: Url,
    timeout: Duration,
    fail_counter: usize,
    filter: Option<fn(&str) -> bool>,
}

impl HlsWatch {
    /// Filter will filter any url that returns `false`, if `None` it will not filter anything.
    /// For example if you want filter preloading segments use: `|e| !(e.contains("preloading"))`.
    pub fn new(
        request: Request,
        http: Client,
        filter: Option<fn(&str) -> bool>,
    ) -> (Self, UnboundedReceiver<HlsQueue>) {
        let (tx, rx) = unbounded_channel();
        let master_url = request
            .url()
            .join(".")
            .expect("Could not join url with '.'.");
        (
            HlsWatch {
                tx,
                request,
                http,
                links: PatriciaSet::new(),
                master_url,
                timeout: Duration::from_secs(10),
                fail_counter: 0,
                filter,
            },
            rx,
        )
    }

    pub async fn run(mut self) -> Result<(), Error> {
        info!("STARTING WATCH!");

        loop {
            if self.fail_counter > HLS_MAX_RETRIES {
                // There have either been errors or no new segments
                // for `HLS_MAX_RETRIES` times the segment duration given
                // in the m3u8 playlist file.
                if self.tx.send(HlsQueue::StreamOver).is_err() {
                    return Err(Error::TIO(std::io::Error::last_os_error()));
                };
                break;
            }

            // Clone the request so we can reuse it in the loop.
            let req = clone_request(&self.request, self.timeout);
            let res = match self.http.execute(req).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("[HLS] Playlist download failed!\n{}", e);
                    self.fail_counter += 1;
                    continue;
                }
            };

            let m3u8_string = match res.text().await {
                Ok(t) => t,
                Err(e) => {
                    warn!("[HLS] Playlist text failed!\n{}", e);
                    self.fail_counter += 1;
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
                    self.fail_counter += 1;
                    continue;
                }
            };

            // Get the target duration of a segment
            let target_duration = m3u8.target_duration;

            // Makes a iterator with the url parts from the playlist
            for e in m3u8.segments.iter().map(|(_, e)| e.uri().trim()) {
                trace!("[HLS] Tries to inserts: {}", e);
                // Check if we have the segment in our set already
                if self.links.insert(e) {
                    // Reset the counter as we got a new segment.
                    self.fail_counter = 0;

                    // Construct a url from the master and the segment.
                    let url_formatted = if let Ok(u) = Url::parse(e) {
                        u
                    } else {
                        // Attempt to parse the url as a relative url.
                        Url::parse(&format!("{}{}", self.master_url.as_str(), &e)).expect(
                            "The m3u8 does not currently work with stream_lib, \
                             please report the issue on the github repo, with an \
                             example of the file if possible.",
                        )
                    };

                    // Check that the filter runs.
                    if self.filter.map_or(true, |f| f(e)) {
                        info!("[HLS] Adds {}!", url_formatted);
                        // Add the segment to the queue.
                        if self.tx.send(HlsQueue::Url(url_formatted)).is_err() {
                            return Err(Error::TIO(std::io::Error::last_os_error()));
                        };
                    }
                }
            }

            if m3u8.has_end_list {
                tracing::debug!("List has end, no more segments expected.");
                break;
            }

            trace!("[HLS] Sleeps for {:#?}", target_duration);
            // Sleeps for the target duration.
            tokio::time::sleep(target_duration).await;
            self.fail_counter += 1;
        }

        Ok(())
    }
}
