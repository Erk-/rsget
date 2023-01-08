use bytes::Bytes;
use futures_core::stream::Stream;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

/// This struct implments a stream that is used to
/// received data from chunked and hls streams.
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
