

mod watch;

use reqwest::{Client as ReqwestClient, Request, Url};

use tokio::io::AsyncWriteExt;
use tokio::io::{AsyncWrite, BufWriter};
use tokio::sync::mpsc::UnboundedReceiver;

#[allow(unused_imports)]
use tracing::{info, trace, warn};


use futures_util::StreamExt;


use crate::error::Error;

use watch::{
    HlsWatch,
    HlsQueue,
    WRITE_SIZE,
};


pub struct HlsDownloader {
    http: ReqwestClient,
    rx: UnboundedReceiver<HlsQueue>,
    watch: HlsWatch,
    headers: reqwest::header::HeaderMap,
}

impl HlsDownloader {
    pub fn new(request: Request, http: ReqwestClient) -> Self {
        let headers = request.headers().clone();
        let (watch, rx) = HlsWatch::new(request, http.clone());
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
        info!("STARTING DOWNLOAD!");
        let mut total_size = 0;
        let mut rx = self.rx;
        let watch = self.watch;
        let mut buf_writer = BufWriter::with_capacity(WRITE_SIZE, writer);

        // TODO: Maybe clean this up after closing the function.
        tokio::task::spawn(watch.run());
        while let Some(hls) = rx.recv().await {
            //println!("GOT ELEMENT");
            match hls {
                HlsQueue::Url(u) => {
                    // These two statements are not part of the spinner.
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
                    )
                    .await?;

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

#[inline]
async fn download_to_file<AW>(
    client: &ReqwestClient,
    mut request: Request,
    writer: &mut BufWriter<AW>,
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
        tsize += size;
    }

    info!("[MASTER] Downloaded: {}", tsize);
    Ok(tsize)
}
