use bytes::Bytes;
use futures_core::stream::Stream;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

#[derive(Debug)]
pub enum StreamType {
    Hls,
    HlsNamed,
    Http,
}

#[derive(Debug)]
pub struct DownloadStream {
    rx: UnboundedReceiver<Event>,
}

impl DownloadStream {
    pub(crate) fn new() -> (Self, UnboundedSender<Event>) {
        let (tx, rx) = unbounded_channel();
        (DownloadStream { rx }, tx)
    }
}

impl DownloadStream {
    pub fn stream_type(&self) -> StreamType {
        StreamType::Hls
    }
}

impl Stream for DownloadStream {
    type Item = Event;

    fn poll_next(
        mut self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        self.rx.poll_recv(cx)
    }
}

#[derive(Debug)]
pub enum Event {
    /// Bytes to be written to
    Bytes {
        bytes: Bytes,
    },
    End,
    Error {
        error: crate::Error,
    },
}
