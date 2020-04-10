use tokio::io::AsyncWrite;

use futures_util::StreamExt;

use reqwest::{Client as ReqwestClient, Request};

use crate::error::Error;

/// A Enum with the types of streams supported
#[derive(Debug)]
pub enum StreamType {
    /// A stream that is just a chunked http response.
    Chuncked(Request),
    /// A m3u8 playlist, which may be a stream.
    HLS(Request),
    /// A m3u8 master playlist and a string which is the name of the stream to download.
    NamedPlaylist(Request, String),
}

#[derive(Debug)]
pub struct Stream {
    stream_type: StreamType,
}

impl Stream {
    /// Creates a new stream handler.
    pub fn new(stream_type: StreamType) -> Self {
        Self { stream_type }
    }
    /// Writes the stream to a writer.
    pub async fn write_file<AW>(self, client: &ReqwestClient, writer: AW) -> Result<u64, Error>
    where
        AW: AsyncWrite + Unpin,
    {
        match self.stream_type {
            StreamType::Chuncked(_) => self.chunked(client, writer).await,
            StreamType::HLS(_) => self.hls(client, writer).await,
            StreamType::NamedPlaylist(_, _) => self.named_playlist(client, writer).await,
        }
    }

    async fn chunked<AW>(self, client: &ReqwestClient, mut writer: AW) -> Result<u64, Error>
    where
        AW: AsyncWrite + Unpin,
    {
        let req = self.get_request();
        let mut stream = client.execute(req).await?.bytes_stream();
        let mut size = 0;
        while let Some(item) = stream.next().await {
            size += tokio::io::copy(&mut item?.as_ref(), &mut writer).await?;
        }

        info!("[MASTER] Downloaded: {}", size);
        Ok(size)
    }

    // This currently clones the client to get a client to run the inner calls as well.
    async fn hls<AW>(self, client: &ReqwestClient, writer: AW) -> Result<u64, Error>
    where
        AW: AsyncWrite + Unpin,
    {
        if let StreamType::HLS(req) = self.stream_type {
            let downloader = crate::hls::HlsDownloader::new(req, client.clone());
            Ok(downloader.download(writer).await?)
        } else {
            unreachable!();
        }
    }

    // This currently clones the client to get a client to run the inner calls as well.
    async fn named_playlist<AW>(self, client: &ReqwestClient, writer: AW) -> Result<u64, Error>
    where
        AW: AsyncWrite + Unpin,
    {
        if let StreamType::NamedPlaylist(req, name) = self.stream_type {
            let downloader = crate::named_hls::NamedHlsDownloader::new(req, client.clone(), name);
            Ok(downloader.download(writer).await?)
        } else {
            unreachable!()
        }
    }

    fn get_request(self) -> Request {
        match self.stream_type {
            StreamType::Chuncked(req) => req,
            StreamType::HLS(req) => req,
            StreamType::NamedPlaylist(req, _) => req,
        }
    }
}
