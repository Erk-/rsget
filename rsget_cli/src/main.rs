//extern crate pretty_env_logger;
extern crate flexi_logger;
extern crate log;
extern crate rsget_lib;
extern crate structopt;

use std::boxed::Box;
use std::fs::File;
use std::path::Path;
use std::process::Command;

use flexi_logger::{opt_format, Logger};
use rsget_lib::Streamable;
use structopt::StructOpt;

use rsget_lib::utils::error::StreamError;
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

fn main() {
    Logger::with_env()
        .format(opt_format)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
    let _ = try_main().map_err(|why| {
        println!("Error running: {:?}", why);
    });
}

fn try_main() -> Result<(), StreamError> {
    //pretty_env_logger::init();
    let opt = Opt::from_args();
    let url = opt.url;
    let stream: Box<Streamable + Send> = rsget_lib::utils::sites::get_site(&url)?;

    // if !stream.is_online() {
    //     return Err(StreamError::Rsget(RsgetError::Offline));
    // }

    if opt.info {
        println!("{:#?}", stream.get_stream()?);
        return Ok(());
    }

    if opt.play && !opt.network_play {
        let status = Command::new("mpv")
            .arg("--no-ytdl")
            .arg(stream_type_to_url(stream.get_stream()?))
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
        let file_name = opt.filename.unwrap_or_else(|| stream.get_default_name());
        let full_path = format!("{}{}", path, strip_characters(&file_name, "<>:\"/\\|?*\0"));
        let path = Path::new(&full_path);
        let file = Box::new(File::create(path)?);
        let size = stream.download(file)?;
        println!("Downloaded: {} MB", size as f64 / 1000.0 / 1000.0);
        Ok(())
    }
}

#[allow(clippy::boxed_local)]
fn stream_network<S>(stream: Box<S>) -> Result<u64, StreamError>
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
    let size = stream.download(socket)?;
    Ok(size)
}

fn strip_characters(original: &str, to_strip: &str) -> String {
    original
        .chars()
        .filter(|&c| !to_strip.contains(c))
        .collect()
}
