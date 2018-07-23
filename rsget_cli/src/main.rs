extern crate clap;
//extern crate pretty_env_logger;
extern crate flexi_logger;
//#[macro_use]
extern crate log;
extern crate rsget_lib;
extern crate hyper;
extern crate hyper_tls;
 
use rsget_lib::Streamable;
use clap::{App, Arg}; //, SubCommand};
use std::process::Command;
use flexi_logger::{Logger,opt_format};

use rsget_lib::utils::error::StreamError;
use rsget_lib::utils::error::RsgetError;

fn main() -> Result<(), StreamError> {
    //pretty_env_logger::init();
    Logger::with_env()
        .format(opt_format)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

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
    let https = hyper_tls::HttpsConnector::new(4).unwrap();
    let client = hyper::Client::builder()
        .build::<_, hyper::Body>(https);

    let stream: Box<Streamable> = rsget_lib::utils::sites::get_site(&client, &url)?;
    
    if !stream.is_online() {
        return Err(StreamError::Rsget(RsgetError::new("Stream is offline")))
    }

    if matches.is_present("info") {
        println!("{}", stream.get_stream());
        return Ok(())
    }

    if matches.is_present("play") {
        let status = Command::new("mpv")
            .arg("--no-ytdl")
            .arg(stream.get_stream())
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

    stream.download(
        format!("{}{}", path, strip_characters(&file_name, "<>:\"/\\|?*\0")),
    )
}

fn strip_characters(original: &str, to_strip: &str) -> String {
    original
        .chars()
        .filter(|&c| !to_strip.contains(c))
        .collect()
}
