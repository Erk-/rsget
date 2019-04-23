//extern crate pretty_env_logger;
extern crate flexi_logger;
extern crate log;
extern crate rsget_lib;
extern crate structopt;
 
use rsget_lib::Streamable;
use std::process::Command;
use flexi_logger::{Logger,opt_format};
use structopt::StructOpt;

use rsget_lib::utils::error::StreamError;
use rsget_lib::utils::error::RsgetError;
use rsget_lib::utils::stream_type_to_url;


#[derive(Debug, StructOpt)]
#[structopt(name = "rsget")]
struct Opt {
    #[structopt(short = "P", long = "play")]
    play: bool,
    #[structopt(short = "i", long = "info")]
    info: bool,
    #[structopt(short = "O", long = "path", default_value = "./")]
    path: String,
    #[structopt(short = "o", long = "output")]
    filename: Option<String>,
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
    let stream: Box<Streamable> = rsget_lib::utils::sites::get_site(&url)?;
    
    if !stream.is_online() {
        return Err(StreamError::Rsget(RsgetError::new("Stream is offline")))
    }

    if opt.info {
        println!("{:#?}", stream.get_stream()?);
        return Ok(())
    }

    if opt.play {
        let status = Command::new("mpv")
            .arg("--no-ytdl")
            .arg(stream_type_to_url(stream.get_stream()?))
            .status()
            .expect("Mpv failed to start");
        std::process::exit(status.code().unwrap())
    }

    let path = opt.path;
    let file_name = String::from(
        opt
            .filename
            .unwrap_or(stream.get_default_name()),
    );

    let size = stream.download(
        format!("{}{}", path, strip_characters(&file_name, "<>:\"/\\|?*\0")),
    )?;
    println!("Downloaded: {} MB", size as f64 / 1000.0 / 1000.0);
    Ok(())
}

fn strip_characters(original: &str, to_strip: &str) -> String {
    original
        .chars()
        .filter(|&c| !to_strip.contains(c))
        .collect()
}
