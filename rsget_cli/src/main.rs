use std::boxed::Box;
use std::fs::File;
use std::path::Path;
use std::process::Command;

use flexi_logger::{opt_format, Logger};
use rsget_lib::{Streamable, Status};
use structopt::StructOpt;
use tokio::prelude::*;
use log::{warn, info};

use rsget_lib::utils::error::StreamError;
use rsget_lib::utils::stream_type_to_url;
use rsget_lib::utils::error::RsgetError;


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
#[tokio::main]
async fn main() -> Result<(), StreamError> {
    Logger::with_env()
        .format(opt_format)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    let opt = Opt::from_args();
    let url = opt.url;
    let stream: Box<dyn Streamable + Send> = rsget_lib::utils::sites::get_site(&url).await?;

    match stream.is_online().await? {
        Status::Offline => return Err(StreamError::Rsget(RsgetError::Offline)),
        Status::Online => (),
        Status::Unknown => {
            warn!("Not sure if stream is online, but will try");
            ()
        },
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

    if opt.network_play {
        use std::thread;
        let child = thread::spawn(move || stream_network(stream));
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
        let _ = child.join();
        Ok(())
    } else {
        let path = opt.path;
        let file_name = opt.filename.unwrap_or(stream.get_default_name().await?);
        let full_path = format!("{}{}", path, strip_characters(&file_name, "<>:\"/\\|?*\0"));
        let path = Path::new(&full_path);
        let file = Box::new(File::create(path)?);
        let size = stream.download(file).await?;
        println!("Downloaded: {} MB", size as f64 / 1000.0 / 1000.0);
        Ok(())
    }
}

#[allow(clippy::boxed_local)]
async fn stream_network<S>(stream: Box<S>) -> Result<u64, StreamError>
where
    S: Streamable + Send + ?Sized,
{
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:61337")?;
    let socket = match listener.accept() {
        Ok((socket, _addr)) => Box::new(socket),
        Err(e) => return Err(e.into()),
    };
    println!("Starts download!");
    let size = stream.download(socket).await?;
    Ok(size)
}

fn strip_characters(original: &str, to_strip: &str) -> String {
    original
        .chars()
        .filter(|&c| !to_strip.contains(c))
        .collect()
}
