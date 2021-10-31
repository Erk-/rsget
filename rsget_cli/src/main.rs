use std::boxed::Box;
use std::path::Path;
use std::process::Command;
use tokio::fs::File;

use rsget_lib::{Status, Streamable};
use structopt::StructOpt;
use tracing::warn;

use rsget_lib::utils::error::{RsgetError, StreamError, StreamResult};
use rsget_lib::utils::stream_type_to_url;

#[derive(Debug, StructOpt)]
#[structopt(name = "rsget")]
struct Opt {
    #[structopt(short = "p", long = "play")]
    play: bool,
    #[structopt(short = "i", long = "info")]
    info: bool,
    #[structopt(short = "O", long = "path", default_value = "./")]
    path: String,
    #[structopt(short = "o", long = "output")]
    filename: Option<String>,
    #[structopt(short = "n", long = "network-play")]
    network_play: bool,
    url: String,
}

use tokio::runtime::Runtime;
fn main() -> StreamResult<()> {
    let runtime = Runtime::new()?;
    runtime.block_on(async move { async_main().await })?;
    runtime.shutdown_timeout(std::time::Duration::from_millis(100));
    Ok(())
}

async fn async_main() -> StreamResult<()> {
    tracing_subscriber::fmt::init();

    let opt = Opt::from_args();
    let url = opt.url;
    let stream: Box<dyn Streamable + Send> = rsget_lib::utils::sites::get_site(&url).await?;

    match stream.is_online().await? {
        Status::Offline => return Err(StreamError::Rsget(RsgetError::Offline)),
        Status::Online => (),
        Status::Unknown => {
            warn!("Not sure if stream is online, but will try");
        }
    }

    if opt.info {
        println!("{:#?}", stream.get_stream().await?);
        return Ok(());
    }

    if opt.play && !opt.network_play {
        let status = Command::new("mpv")
            .arg("--no-ytdl")
            .arg(stream_type_to_url(stream.get_stream().await?))
            .status()
            .expect("Mpv failed to start");
        std::process::exit(status.code().unwrap())
    }

    /*
    if opt.network_play {
        let child = tokio::spawn(stream_network(stream));
        if opt.play {
            Command::new("mpv")
                .arg("--no-ytdl")
                .arg("--cache=8192")
                .arg("tcp://127.0.0.1:61337")
                .status()
                .expect("Mpv failed to start");
        } else {
            println!("Connect player to <tcp://127.0.0.1:61337>");
        }
        let _ = child.await;
        Ok(())
    } else {
    */
    let path = opt.path;
    let file_name = opt.filename.unwrap_or(stream.get_default_name().await?);
    let full_path = format!("{}{}", path, strip_characters(&file_name, "<>:\"/\\|?*\0"));
    let path = Path::new(&full_path);
    let file = Box::new(File::create(path).await?);
    let st = stream.get_stream().await?;
    let dl = stream_lib::Stream::new(st);
    let http = reqwest::Client::new();
    let size = dl.write_file(&http, file).await?;
    println!("Downloaded: {} MB", size as f64 / 1000.0 / 1000.0);
    Ok(())
    //}
}

/*
#[allow(clippy::boxed_local)]
async fn stream_network<S>(stream: Box<S>) -> Result<u64, StreamError>
where
    S: Streamable + Send + ?Sized,
{
    use tokio::net::TcpListener;
    let mut listener = TcpListener::bind("127.0.0.1:61337").await?;
    let socket = match listener.accept().await {
        Ok((socket, _addr)) => Box::new(socket),
        Err(e) => return Err(e.into()),
    };
    println!("Starts download!");
    let st = stream.get_stream().await?;
    let dl = stream_lib::Stream::new(st);
    let http = reqwest::Client::new();
    let size = dl.write_file(&http, socket).await?;
    Ok(size)
}
*/

fn strip_characters(original: &str, to_strip: &str) -> String {
    original
        .chars()
        .filter(|&c| !to_strip.contains(c))
        .collect()
}
