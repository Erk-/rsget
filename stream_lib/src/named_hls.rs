/// Write buffer
pub const WRITE_SIZE: usize = 131_072;

/// HLS will try and look for new segments 12 times,
pub const HLS_MAX_RETRIES: usize = 12;

use reqwest::{Client as ReqwestClient, Request, Url};

use hls_m3u8::MasterPlaylist;
use hls_m3u8::MediaPlaylistOptions;

use tokio::io::AsyncWriteExt;
use tokio::io::{AsyncWrite, BufWriter};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use futures_util::StreamExt;

use std::collections::HashSet;
use std::time::Duration;

use crate::error::Error;

#[derive(Clone)]
enum HlsQueue {
    Url(Url),
    StreamOver,
}

pub struct NamedHlsDownloader {
    http: ReqwestClient,
    rx: UnboundedReceiver<HlsQueue>,
    watch: NamedHlsWatch,
    headers: reqwest::header::HeaderMap,
}

impl NamedHlsDownloader {
    pub fn new(request: Request, http: ReqwestClient, name: String) -> Self {
        let headers = request.headers().clone();
        let (watch, rx) = NamedHlsWatch::new(request, http.clone(), name);
        Self {
            http,
            rx,
            watch,
            headers,
        }
    }

    pub async fn download<AW>(self, writer: AW) -> Result<u64, Error>
    where
        AW: AsyncWrite + Unpin,
    {
        let mut total_size = 0;
        let mut rx = self.rx;
        let watch = self.watch;
        let mut buf_writer = BufWriter::with_capacity(WRITE_SIZE, writer);
        // TODO: Maybe clean this up after closing the function.
        tokio::task::spawn_blocking(|| watch.run());
        while let Some(hls) = rx.recv().await {
            match hls {
                HlsQueue::Url(u) => {
                    let req = self.http.get(u).headers(self.headers.clone()).build()?;
                    let size = download_to_file(&self.http, req, &mut buf_writer).await?;
                    total_size += size;
                }
                HlsQueue::StreamOver => break,
            }
        }
        buf_writer.flush().await?;
        Ok(total_size)
    }
}

struct NamedHlsWatch {
    tx: UnboundedSender<HlsQueue>,
    request: Request,
    http: ReqwestClient,
    links: HashSet<String>,
    master_url: Url,
    name: String
}

impl NamedHlsWatch {
    fn new(request: Request, http: ReqwestClient, name: String) -> (Self, UnboundedReceiver<HlsQueue>) {
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
                links: HashSet::new(),
                master_url,
                name,
            },
            rx,
        )
    }

    async fn run(mut self) -> Result<(), Error> {
        let mut counter = 0;

        loop {
            if counter > HLS_MAX_RETRIES {
                // There have either been errors or no new segments
                // for `HLS_MAX_RETRIES` times the segment duration given
                // in the m3u8 playlist file.
                if let Err(_) = self.tx.send(HlsQueue::StreamOver) {
                    return Err(Error::TIO(std::io::Error::last_os_error()));
                };
                break;
            }

            // Use the same headers as the original request
            let req = match self.request.try_clone() {
                Some(r) => r,
                // If the body is not able to be cloned it will only clone the headers.
                None => {
                    warn!("[HLS] body not able to be cloned only clones headers.");
                    match self
                        .http
                        .get(self.request.url().clone())
                        .headers(self.request.headers().clone())
                        .build()
                    {
                        Ok(br) => br,
                        Err(e) => {
                            warn!("[HLS] Request creation failed!\n{}", e);
                            counter += 1;
                            continue;
                        }
                    }
                }
            };

            let master_res = match self.http.execute(req).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("[HLS] Playlist download failed!\n{}", e);
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

            let master_playlist = master_string.parse::<MasterPlaylist>().unwrap();

            let segment_pos = master_playlist
                .media_tags()
                .iter()
                .position(|e| e.name().as_ref() == self.name)
                .unwrap_or(0);

            #[allow(clippy::redundant_closure)]
            let master_iter: Vec<String> = master_playlist
                .stream_inf_tags()
                .iter()
                .map(|e| e.uri())
                .map(|e| String::from(e.trim()))
                .collect();

            let segment = master_iter[segment_pos].clone();

            let mp_hls = match self.http
                .get(&segment)
                .headers(self.request.headers().clone())
                .build()
            {
                Ok(p) => p,
                Err(e) => {
                    warn!("[HLS] URI!\n{}", e);
                    trace!("[HLS]\n{}", segment);
                    counter += 1;
                    continue;
                }
            };

            let res = match self.http.execute(mp_hls).await {
                Ok(r) => r,
                Err(e) => {
                    warn!("[HLS] Playlist download failed!\n{}", e);
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

            // Allow excess segment duration because a lot of video sites have
            // not very high quality m3u8 playlists, where the video segments,
            // may be longer than what the file specifies as max.
            let m3u8 = match MediaPlaylistOptions::new()
                .allowable_excess_segment_duration(Duration::from_secs(10))
                .parse(&m3u8_string)
            {
                Ok(p) => p,
                Err(e) => {
                    warn!("[HLS] Parsing failed!\n{}", e);
                    trace!("[HLS]\n{}", &m3u8_string);
                    counter += 1;
                    continue;
                }
            };

            // Get the target duration of a segment
            let target_duration = m3u8.target_duration_tag().duration();

            // Makes a iterator with the url parts from the playlist
            let m3u8_iterator = m3u8.segments().iter().map(|e| String::from(e.uri().trim()));

            for e in m3u8_iterator {
                trace!("[HLS] Tries to inserts: {}", e);
                // Check if we have the segment in our set already
                if self.links.insert(e.clone()) {
                    // Reset the counter as we got a new segment.
                    counter = 0;

                    // Construct a url from the master and the segment.
                    let url_formatted = if let Ok(u) = Url::parse(&e) {
                        u
                    } else {
                        Url::parse(&format!("{}{}", self.master_url.as_str(), &e))
                            .expect("The m3u8 does not currently work with stream_lib, please report the issue on the github repo, with an example of the playlistfile.")
                    };

                    // Check if the segment is a Afreeca preloading segment.
                    if !(e.contains("preloading")) {
                        info!("[HLS] Adds {}!", url_formatted);
                        // Add the segment to the queue.
                        if let Err(_) = self.tx.send(HlsQueue::Url(url_formatted)) {
                            return Err(Error::TIO(std::io::Error::last_os_error()));
                        };
                    }
                }
            }
            warn!("[HLS] Sleeps for {:#?}", target_duration);
            // Sleeps for the target duration.
            tokio::time::delay_for(target_duration).await;
            counter += 1;
        }

        Ok(())
    }
}

#[inline]
async fn download_to_file<AW>(
    client: &ReqwestClient,
    request: Request,
    writer: &mut BufWriter<AW>,
) -> Result<u64, Error>
where
    AW: AsyncWrite + Unpin,
{
    let mut stream = client.execute(request).await?.bytes_stream();
    let mut size = 0;
    while let Some(item) = stream.next().await {
        size += tokio::io::copy(&mut item?.as_ref(), writer).await?;
    }

    info!("[MASTER] Downloaded: {}", size);
    Ok(size)
}