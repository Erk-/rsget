mod named_watch;
mod watch;

/// HLS will try and look for new segments 12 times,
pub const HLS_MAX_RETRIES: usize = 12;

use std::time::Duration;

use reqwest::header::HeaderMap;
use reqwest::{Client, Method, Request, Url};

use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender};

#[allow(unused_imports)]
use tracing::{info, trace, warn};

use futures_util::StreamExt;

use crate::download_stream::{DownloadStream, Event};
use crate::error::Error;

use watch::HlsWatch;

use named_watch::NamedHlsWatch;

#[derive(Debug, Clone)]
pub enum HlsQueue {
    Url(Url),
    StreamOver,
}
pub struct HlsDownloader {
    http: Client,
    rx: UnboundedReceiver<HlsQueue>,
    watch: Watcher,
    headers: HeaderMap,
}

enum Watcher {
    Unnamed(HlsWatch),
    Named(NamedHlsWatch),
}

impl Watcher {
    async fn run(self) -> Result<(), Error> {
        match self {
            Watcher::Unnamed(watch) => watch.run().await,
            Watcher::Named(watch) => watch.run().await,
        }
    }
}

impl HlsDownloader {
    pub(crate) fn new(request: Request, http: Client, filter: Option<fn(&str) -> bool>) -> Self {
        let headers = request.headers().clone();
        let (watch, rx) = HlsWatch::new(request, http.clone(), filter);
        Self {
            http,
            rx,
            watch: Watcher::Unnamed(watch),
            headers,
        }
    }

    pub(crate) fn new_named(
        request: Request,
        http: Client,
        name: String,
        filter: Option<fn(&str) -> bool>,
    ) -> Self {
        let headers = request.headers().clone();
        let (watch, rx) = NamedHlsWatch::new(request, http.clone(), name, filter);
        Self {
            http,
            rx,
            watch: Watcher::Named(watch),
            headers,
        }
    }

    pub(crate) fn download(self) -> DownloadStream {
        info!("STARTING DOWNLOAD!");
        let rx = self.rx;
        let watch = self.watch;

        // TODO: Maybe clean this up after closing the function.
        tokio::task::spawn(watch.run());

        let (download_stream, event_tx) = DownloadStream::new();

        tokio::task::spawn(bytes_forwarder(self.http, self.headers, rx, event_tx));

        download_stream
    }
}

async fn bytes_forwarder(
    http: Client,
    headers: HeaderMap,
    mut hls_rx: UnboundedReceiver<HlsQueue>,
    event_tx: UnboundedSender<Event>,
) {
    while let Some(hls) = hls_rx.recv().await {
        //println!("GOT ELEMENT");
        match hls {
            HlsQueue::Url(u) => {
                // These two statements are not part of the spinner.
                const TIMEOUT: Duration = Duration::from_secs(10);
                let req = http
                    .get(u)
                    .headers(headers.clone())
                    .timeout(TIMEOUT)
                    .build()
                    .unwrap();
                if let Err(error) = download_to_file(
                    http.clone(),
                    req,
                    event_tx.clone(),
                    Some(TIMEOUT),
                    Some(TIMEOUT),
                )
                .await
                {
                    if let Err(error) = event_tx.send(Event::Error { error }) {
                        tracing::warn!("Could not send event: {}", error);
                    };
                }
            }
            HlsQueue::StreamOver => {
                if let Err(error) = event_tx.send(Event::End) {
                    tracing::warn!("Could not send event: {}", error);
                };
                break;
            }
        }
    }
}

/// Timeout is the total duration the response may take set to none to make it unlimited.
/// Download timeout is the timeout between two chunks of a streaming response.
pub(crate) async fn download_to_file(
    client: Client,
    mut request: Request,
    event_tx: UnboundedSender<Event>,
    timeout: Option<Duration>,
    download_timeout: Option<Duration>,
) -> Result<(), Error> {
    const TIMEOUT_MAX: Duration = Duration::MAX;

    *request.timeout_mut() = timeout;
    let mut stream = client.execute(request).await?.bytes_stream();
    while let Ok(Some(item)) =
        tokio::time::timeout(download_timeout.unwrap_or(TIMEOUT_MAX), stream.next()).await
    {
        match item {
            Ok(bytes) => {
                if let Err(error) = event_tx.send(Event::Bytes { bytes }) {
                    tracing::warn!("Could not send event: {}", error);
                };
            }
            Err(error) => {
                if let Err(error) = event_tx.send(Event::Error {
                    error: error.into(),
                }) {
                    tracing::warn!("Could not send event: {}", error);
                };
            }
        }
    }

    Ok(())
}

pub fn clone_request(request: &Request, timeout: Duration) -> Request {
    if let Some(mut r) = request.try_clone() {
        *r.timeout_mut() = Some(timeout);
        r
    } else {
        warn!("[HLS] body not able to be cloned only clones headers.");
        let mut r = Request::new(Method::GET, request.url().clone());
        *r.headers_mut() = request.headers().clone();
        *r.timeout_mut() = Some(timeout);
        r
    }
}
