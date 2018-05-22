extern crate rsget_hls;
extern crate pretty_env_logger;
#[macro_use] extern crate log;
extern crate tokio;
extern crate rsget_lib;

use rsget_hls::*;
use rsget_lib::utils::error::StreamError;

fn main() {
    pretty_env_logger::init();
    let fut = download_to_file("https://www.dr.dk/", "./test.html");
    tokio::run(fut);
    println!("{}", fut);
    //hls_download(String::from("https://www.dr.dk/"), String::from("test"));
}
