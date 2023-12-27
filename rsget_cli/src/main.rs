use std::{
    path::{Path, PathBuf},
    process::Command,
    time::Duration,
};

use clap::Parser;
use futures_util::StreamExt as _;
use reqwest::Url;
use rsget_lib::{
    utils::error::{RsgetError, StreamError, StreamResult},
    Status, Streamable,
};
use tokio::{
    fs::File,
    io::{AsyncWriteExt as _, BufWriter},
    runtime::Runtime,
};
use tracing::warn;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Opt {
    #[arg(short = 'p', long = "play")]
    play: bool,
    #[arg(short = 'i', long = "info")]
    info: bool,
    #[arg(short = 'O', long = "folder", default_value = "./")]
    folder: PathBuf,
    #[arg(short = 'o', long = "output")]
    filename: Option<String>,
    #[arg(short = 'n', long = "network-play")]
    network_play: bool,
    url: String,
}

fn main() -> StreamResult<()> {
    let runtime = Runtime::new()?;
    runtime.block_on(async move { async_main().await })?;
    runtime.shutdown_timeout(Duration::from_millis(100));
    Ok(())
}

async fn async_main() -> StreamResult<()> {
    tracing_subscriber::fmt::init();

    let opt = Opt::parse();
    let url = opt.url;
    let parsed_url = Url::parse(&url).unwrap();
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
            .arg(parsed_url.as_str())
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

    let folder = opt.folder;
    if !folder.is_dir() {
        eprintln!(
            "The --folder argument was not a directonary. ({:?})",
            folder
        );
        return Ok(());
    }

    let file_name = strip_characters(
        &opt.filename.unwrap_or(stream.get_default_name().await?),
        "<>:\"/\\|?*\0",
    );
    let full_path = folder.join(file_name);
    let path = Path::new(&full_path);
    let mut file = BufWriter::new(File::create(path).await?);
    let mut dl = stream.get_stream().await?;

    let spinsty = indicatif::ProgressStyle::default_spinner()
        .template(
            "{spinner} Elapsed time: {elapsed_precise}, {.blue}Total download: {bytes:30.yellow}",
        )
        .unwrap();
    let spinner = indicatif::ProgressBar::new_spinner().with_style(spinsty);

    let mut size = 0;

    while let Some(event) = dl.next().await {
        match event {
            stream_lib::Event::Bytes { mut bytes } => {
                spinner.inc(bytes.len() as u64);
                size += bytes.len();
                file.write_all_buf(&mut bytes).await?;
            }
            stream_lib::Event::End => {
                eprintln!("End received");
                break;
            }
            stream_lib::Event::Error { error } => {
                eprintln!("Error occured when downloading stream: {}", error);
                break;
            }
        }
    }

    println!("Downloaded: {} MB", size as f64 / 1000.0 / 1000.0);
    Ok(())
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

/// Strip characters not allowed on some file systems such as Posix
/// and NTFS.
fn strip_characters(original: &str, to_strip: &str) -> String {
    original
        .chars()
        .filter(|&c| !to_strip.contains(c))
        .collect()
}

fn get_current_working_dir() -> PathBuf {
    std::env::current_dir().unwrap()
}
