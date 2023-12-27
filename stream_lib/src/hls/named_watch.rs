use std::time::Duration;

use hls_m3u8::{tags::VariantStream, MasterPlaylist, MediaPlaylist};
use patricia_tree::PatriciaSet;
use reqwest::{Client, Request, Url};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};
use tracing::{debug, trace, warn};

use crate::{
    hls::{clone_request, HLS_MAX_RETRIES},
    Error,
};

use super::HlsQueue;

pub struct NamedHlsWatch {
    tx: UnboundedSender<HlsQueue>,
    request: Request,
    http: Client,
    links: PatriciaSet,
    master_url: Url,
    timeout: Duration,
    name: Option<String>,
    filter: Option<fn(&str) -> bool>,
}

impl NamedHlsWatch {
    pub(crate) fn new(
        request: Request,
        http: Client,
        name: String,
        filter: Option<fn(&str) -> bool>,
    ) -> (Self, UnboundedReceiver<HlsQueue>) {
        let (tx, rx) = unbounded_channel();
        let master_url = request
            .url()
            .clone()
            .join(".")
            .expect("Could not join url with '.'.");
        (
            Self {
                tx,
                request,
                http,
                links: PatriciaSet::new(),
                timeout: Duration::from_secs(10),
                master_url,
                name: Some(name),
                filter,
            },
            rx,
        )
    }

    pub(crate) fn new_first(
        request: Request,
        http: Client,
        filter: Option<fn(&str) -> bool>,
    ) -> (Self, UnboundedReceiver<HlsQueue>) {
        let (tx, rx) = unbounded_channel();
        let master_url = request
            .url()
            .clone()
            .join(".")
            .expect("Could not join url with '.'.");
        (
            Self {
                tx,
                request,
                http,
                links: PatriciaSet::new(),
                timeout: Duration::from_secs(10),
                master_url,
                name: None,
                filter,
            },
            rx,
        )
    }

    pub async fn run(mut self) -> Result<(), Error> {
        let mut counter = 0;

        loop {
            if counter > HLS_MAX_RETRIES {
                // There have either been errors or no new segments
                // for `HLS_MAX_RETRIES` times the segment duration given
                // in the m3u8 playlist file.
                if self.tx.send(HlsQueue::StreamOver).is_err() {
                    return Err(Error::TIO(std::io::Error::last_os_error()));
                };
                break;
            }

            // Use the same headers as the original request
            let req = clone_request(&self.request, self.timeout);

            let master_res = match self.http.execute(req).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("[HLS] Master playlist download failed!\n{}", e);
                    counter += 1;
                    continue;
                }
            };

            let master_string = match master_res.text().await {
                Ok(t) => t,
                Err(e) => {
                    warn!("[HLS] Playlist text failed!\n{}", e);
                    counter += 1;
                    continue;
                }
            };

            let master_playlist = match MasterPlaylist::try_from(master_string.as_str()) {
                Ok(mp) => mp,
                Err(e) => {
                    warn!("[HLS] Master playlist parsing failed: {}", e);
                    counter += 1;
                    continue;
                }
            };

            let ext_media = if let Some(name) = &self.name {
                match master_playlist.media.iter().find(|e| e.name() == name) {
                    Some(em) => Some(em),
                    None => {
                        counter += 1;
                        continue;
                    }
                }
            } else {
                None
            };

            let media = if let Some(ext_media) = ext_media {
                match master_playlist
                    .variant_streams
                    .iter()
                    .find(|e| e.is_associated(ext_media))
                {
                    Some(m) => m,
                    None => {
                        counter += 1;
                        continue;
                    }
                }
            } else {
                match master_playlist.variant_streams.iter().next() {
                    Some(m) => m,
                    None => {
                        counter += 1;
                        continue;
                    }
                }
            };

            let uri = match media {
                VariantStream::ExtXIFrame { uri: u, .. } => u,
                VariantStream::ExtXStreamInf { uri: u, .. } => u,
            };

            if let Ok(u) = Url::parse(uri) {
                self.master_url = u.join(".").expect("Could not join with '.'");
            }

            let uri_formatted = if let Ok(u) = Url::parse(&uri) {
                u
            } else {
                Url::parse(&format!("{}{}", self.master_url.as_str(), &uri))
                    .expect("The m3u8 does not currently work with stream_lib, please report the issue on the github repo, with an example of the playlistfile.")
            };

            let mp_hls = match self
                .http
                .get(uri_formatted.as_ref())
                .headers(self.request.headers().clone())
                .timeout(self.timeout)
                .build()
            {
                Ok(p) => p,
                Err(e) => {
                    debug!("[HLS] URI!\n{}", e);
                    trace!("[HLS]\n{}", media);
                    counter += 1;
                    continue;
                }
            };

            let res = match self.http.execute(mp_hls).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("[HLS] Minor playlist download failed!\n{}", e);
                    counter += 1;
                    continue;
                }
            };

            let m3u8_string = match res.text().await {
                Ok(t) => t,
                Err(e) => {
                    warn!("[HLS] Playlist text failed!\n{}", e);
                    counter += 1;
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
                    counter += 1;
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
                if self.links.insert(&e) {
                    // Reset the counter as we got a new segment.
                    counter = 0;

                    // Construct a url from the master and the segment.
                    let url_formatted = if let Ok(u) = Url::parse(&e) {
                        u
                    } else {
                        Url::parse(&format!("{}{}", self.master_url.as_str(), &e))
                        .expect("The m3u8 does not currently work with stream_lib, please report the issue on the github repo, with an example of the playlistfile.")
                    };

                    // Check that the filter runs.
                    if self.filter.map_or(true, |f| f(&e)) {
                        debug!("[HLS] Adds {}!", url_formatted);
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

            debug!("[HLS] Sleeps for {:#?}", target_duration);
            // Sleeps for the target duration.
            tokio::time::sleep(target_duration).await;
            counter += 1;
        }

        Ok(())
    }
}
