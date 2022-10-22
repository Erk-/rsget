use reqwest::Client;

use futures_util::StreamExt as _;
use tokio::io::AsyncWriteExt;
use stream_lib::Event;

/// Write buffer
pub const WRITE_SIZE: usize = 131_072;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();
    let args = std::env::args().collect::<Vec<_>>();
    let url = args.get(1).expect("Pass a url as the first argument");

    let http = Client::new();
    let req = http.get(url).build()?;
    let mut dl = stream_lib::download_hls(http, req);

    let mut file = tokio::io::BufWriter::with_capacity(WRITE_SIZE, tokio::fs::File::create("./test.mp4").await?);

    while let Some(event) = dl.next().await {
        match event {
            Event::Bytes { bytes } => {
                file.write_all(&bytes).await?;
            },
            Event::End => break,
            Event::Error { error } => {
                eprintln!("Encounted error: {}", error);
                break;
            },
        }
    }
    Ok(())
}
