/// Write buffer
pub const WRITE_SIZE: usize = 131_072;

/// HLS will try and look for new segments 12 times,
pub const HLS_MAX_RETRIES: usize = 12;

use reqwest::{Client as ReqwestClient, Request, Url};

use hls_m3u8::tags::VariantStream;
use hls_m3u8::MasterPlaylist;
use hls_m3u8::MediaPlaylist;

use tokio::io::AsyncWriteExt;
use tokio::io::{AsyncWrite, BufWriter};
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use futures_util::StreamExt;

use patricia_tree::PatriciaSet;

use std::convert::TryFrom;
use std::time::Duration;

#[cfg(feature = "spinner")]
use std::sync::Arc;

use tracing::{info, trace, warn};

use crate::error::Error;

#[derive(Debug, Clone)]
enum HlsQueue {
    Url(Url),
    StreamOver,
}

pub struct NamedHlsDownloader {
    http: ReqwestClient,
    rx: UnboundedReceiver<HlsQueue>,
    watch: NamedHlsWatch,
    headers: reqwest::header::HeaderMap,
    #[cfg(feature = "spinner")]
    progress: Arc<indicatif::MultiProgress>,
}

impl NamedHlsDownloader {
    pub fn new(request: Request, http: ReqwestClient, name: String) -> Self {
        let headers = request.headers().clone();
        let (watch, rx) = NamedHlsWatch::new(request, http.clone(), name);
        #[cfg(feature = "spinner")]
        let progress = Arc::new(indicatif::MultiProgress::new());
        Self {
            http,
            rx,
            watch,
            headers,
            #[cfg(feature = "spinner")]
            progress,
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

        #[cfg(feature = "spinner")]
        {
            // HACK: This is needed until https://github.com/mitsuhiko/indicatif/issues/125 gets resolved.
            let mp_l = self.progress.clone();
            tokio::task::spawn_blocking(move || {
                std::thread::sleep(std::time::Duration::from_millis(500));
                mp_l.join().unwrap();
            });
        }

        #[cfg(feature = "spinner")]
        let spinsty = indicatif::ProgressStyle::default_spinner()
            .template("{spinner.blue} {pos:30.yellow} segments {elapsed_precise}");
        #[cfg(feature = "spinner")]
        let spinner = self.progress.add(indicatif::ProgressBar::new(0));
        #[cfg(feature = "spinner")]
        spinner.set_style(spinsty);

        #[cfg(feature = "spinner")]
        let spinst2 = indicatif::ProgressStyle::default_spinner()
            .template("{.blue}Total download: {bytes:30.yellow}");
        #[cfg(feature = "spinner")]
        let spinner2 = self.progress.add(indicatif::ProgressBar::new(0));
        #[cfg(feature = "spinner")]
        spinner2.set_style(spinst2);

        #[cfg(feature = "spinner")]
        let sty = indicatif::ProgressStyle::default_bar()
            .template("{bar:40.green/yellow} {bytes:>7}/{total_bytes:7}");

        // TODO: Maybe clean this up after closing the function.
        tokio::task::spawn(watch.run());
        while let Some(hls) = rx.recv().await {
            match hls {
                HlsQueue::Url(u) => {
                    #[cfg(feature = "spinner")]
                    spinner.inc(1);
                    #[cfg(feature = "spinner")]
                    let head = self
                        .http
                        .head(u.clone())
                        .timeout(std::time::Duration::from_secs(10))
                        .send()
                        .await?;
                    #[cfg(feature = "spinner")]
                    let csize = if head.status().is_success() {
                        head.headers()
                            .get(reqwest::header::CONTENT_LENGTH)
                            .and_then(|l| l.to_str().ok())
                            .and_then(|l| l.parse().ok())
                            .unwrap_or(0)
                    } else {
                        0
                    };

                    #[cfg(feature = "spinner")]
                    let pb = self.progress.add(indicatif::ProgressBar::new(csize));
                    #[cfg(feature = "spinner")]
                    pb.set_style(sty.clone());

                    let req = self
                        .http
                        .get(u)
                        .headers(self.headers.clone())
                        .timeout(std::time::Duration::from_secs(10))
                        .build()?;
                    let size = download_to_file(
                        &self.http,
                        req,
                        &mut buf_writer,
                        #[cfg(feature = "spinner")]
                        pb,
                    )
                    .await?;

                    #[cfg(feature = "spinner")]
                    spinner2.inc(size);

                    total_size += size;
                }
                HlsQueue::StreamOver => {
                    warn!("Stream ended");

                    break;
                }
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
    links: PatriciaSet,
    master_url: Url,
    name: String,
}

impl NamedHlsWatch {
    fn new(
        request: Request,
        http: ReqwestClient,
        name: String,
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
                if self.tx.send(HlsQueue::StreamOver).is_err() {
                    return Err(Error::TIO(std::io::Error::last_os_error()));
                };
                break;
            }

            // Use the same headers as the original request
            let req = match self.request.try_clone() {
                Some(mut r) => {
                    *r.timeout_mut() = Some(std::time::Duration::from_secs(10));
                    r
                }
                // If the body is not able to be cloned it will only clone the headers.
                None => {
                    warn!("[HLS] body not able to be cloned only clones headers.");
                    match self
                        .http
                        .get(self.request.url().clone())
                        .headers(self.request.headers().clone())
                        .timeout(std::time::Duration::from_secs(10))
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

            let ext_media = match master_playlist
                .media
                .iter()
                .find(|e| e.name() == &self.name)
            {
                Some(em) => em,
                None => {
                    counter += 1;
                    continue;
                }
            };

            let media = match master_playlist
                .variant_streams
                .iter()
                .find(|e| e.is_associated(ext_media))
            {
                Some(m) => m,
                None => {
                    counter += 1;
                    continue;
                }
            };

            let uri = match media {
                VariantStream::ExtXIFrame { uri: u, .. } => u,
                VariantStream::ExtXStreamInf { uri: u, .. } => u,
            };

            if let Ok(u) = Url::parse(uri) {
                self.master_url = u.join(".").expect("Could not join with '.'");
            }

            let mp_hls = match self
                .http
                .get(uri.as_ref())
                .headers(self.request.headers().clone())
                .timeout(std::time::Duration::from_secs(10))
                .build()
            {
                Ok(p) => p,
                Err(e) => {
                    info!("[HLS] URI!\n{}", e);
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
                        if self.tx.send(HlsQueue::Url(url_formatted)).is_err() {
                            return Err(Error::TIO(std::io::Error::last_os_error()));
                        };
                    }
                }
            }
            warn!("[HLS] Sleeps for {:#?}", target_duration);
            // Sleeps for the target duration.
            tokio::time::sleep(target_duration).await;
            counter += 1;
        }

        Ok(())
    }
}

#[inline]
async fn download_to_file<AW>(
    client: &ReqwestClient,
    mut request: Request,
    writer: &mut BufWriter<AW>,
    #[cfg(feature = "spinner")] pb: indicatif::ProgressBar,
) -> Result<u64, Error>
where
    AW: AsyncWrite + Unpin,
{
    if request.timeout().is_none() {
        *request.timeout_mut() = Some(std::time::Duration::from_secs(10));
    }
    let mut stream = client.execute(request).await?.bytes_stream();
    let mut tsize = 0;
    while let Some(item) = stream.next().await {
        let size = tokio::io::copy(&mut item?.as_ref(), writer).await?;
        #[cfg(feature = "spinner")]
        pb.inc(size);
        tsize += size;
    }

    info!("[MASTER] Downloaded: {}", tsize);
    Ok(tsize)
}
