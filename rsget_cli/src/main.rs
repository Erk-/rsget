extern crate clap;
//extern crate pretty_env_logger;
extern crate flexi_logger;
extern crate log;
extern crate rsget_lib;

use std::process::Command;
use std::fs::File;
use std::path::Path;
use std::boxed::Box;
 
use rsget_lib::Streamable;
use clap::{App, Arg}; //, SubCommand};

use flexi_logger::{Logger,opt_format};

use rsget_lib::utils::error::StreamError;
use rsget_lib::utils::error::RsgetError;
use rsget_lib::utils::stream_type_to_url;

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
    let matches = App::new("ruststreamer")
       .version("0.1")
        .author("Valdemar Erk <v@erk.io>")
        .about("Downloads streams")
        .arg(
            Arg::with_name("play")
               .short("P")
                .long("play")
                .takes_value(false),
        )
        .arg(
            Arg::with_name("info")
                .short("i")
                .long("info")
                .help("info")
                .required(false),
        )
        .arg(
            Arg::with_name("path")
                .short("O")
                .long("path")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("filename")
                .short("o")
                .long("output")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("URL")
                .help("The url of the stream")
                .required(true)
                .index(1),
        )
        .get_matches();
    let url = String::from(matches.value_of("URL").unwrap());
    let stream: Box<Streamable> = rsget_lib::utils::sites::get_site(&url)?;
    
    if !stream.is_online() {
        return Err(StreamError::Rsget(RsgetError::new("Stream is offline")))
    }

    if matches.is_present("info") {
        println!("{:#?}", stream.get_stream()?);
        return Ok(())
    }

    if matches.is_present("play") {
        let status = Command::new("mpv")
            .arg("--no-ytdl")
            .arg(stream_type_to_url(stream.get_stream()?))
            .status()
            .expect("Mpv failed to start");
        std::process::exit(status.code().unwrap())
    }

    let path = String::from(matches.value_of("path").unwrap_or("./"));
    let file_name = String::from(
        matches
            .value_of("filename")
            .unwrap_or(&stream.get_default_name()),
    );
    let full_path = format!("{}{}", path, strip_characters(&file_name, "<>:\"/\\|?*\0"));
    let path = Path::new(&full_path);
    let file = Box::new(File::create(path)?);
    let size = stream.download(file)?;
    println!("Downloaded: {} MB", size as f64 / 1000.0 / 1000.0);
    Ok(())
}

fn strip_characters(original: &str, to_strip: &str) -> String {
    original
        .chars()
        .filter(|&c| !to_strip.contains(c))
        .collect()
}
