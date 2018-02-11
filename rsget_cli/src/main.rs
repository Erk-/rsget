extern crate rsget_lib;
extern crate clap;

#[macro_use]
extern crate log;
extern crate env_logger;

use rsget_lib::Streamable;
use clap::{Arg, App}; //, SubCommand};

fn main() {
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
    let panda_stream = rsget_lib::plugins::panda::PandaTv::new(url);
    
    let path = String::from(matches.value_of("path").unwrap_or("./"));
    let file_name = String::from(matches.value_of("filename").unwrap_or(&panda_stream.get_default_name()));
    match panda_stream.download(format!("{}{}",
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
