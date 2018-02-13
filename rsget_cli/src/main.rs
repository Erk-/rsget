extern crate rsget_lib;
extern crate clap;
extern crate tokio_core;

#[macro_use]
extern crate log;
extern crate env_logger;

use rsget_lib::Streamable;
use clap::{Arg, App}; //, SubCommand};
use tokio_core::reactor::Core;

fn main() {
    env_logger::init();
    
    let matches = App::new("ruststreamer")
        .version("0.1")
        .author("Valdemar Erk <v@erk.io>")
        .about("Downloads streams")
        .arg(Arg::with_name("path")
             .short("p")
             .long("path")
             .takes_value(true))
        .arg(Arg::with_name("filename")
             .short("o")
             .long("output")
             .takes_value(true))
        .arg(Arg::with_name("URL")
             .help("The url of the stream")
             .required(true)
             .index(1))
        .get_matches();
    let url = String::from(matches.value_of("URL").unwrap());
    let stream: Box<Streamable> = match rsget_lib::utils::sites::get_site(&url){
        Some(b) => b,
        None => {
            info!("Site not implemented");
            std::process::exit(1)
        },
    };
    
    let path = String::from(matches.value_of("path").unwrap_or("./"));
    let file_name = String::from(matches.value_of("filename")
                                 .unwrap_or(&stream.get_default_name()));
    
    let mut core = match Core::new() {
        Ok(c) => c,
        Err(why) => panic!("why: {}", why),
    };
    
    match stream.download(&mut core, format!("{}{}",
                                  path,
                                  strip_characters(&file_name, "<>:\"/\\|?*\0"))
    ) {
        Some(_) => std::process::exit(0),
        None => {
            info!("Download Failed");
            std::process::exit(1)
        },
    }
}

fn strip_characters(original : &str, to_strip : &str) -> String {
    original.chars().filter(|&c| !to_strip.contains(c)).collect()
}
