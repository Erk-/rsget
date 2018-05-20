extern crate rsget_hls;
extern crate pretty_env_logger;
#[macro_use] extern crate log;

use rsget_hls::hls_download;

fn main() {
    pretty_env_logger::init();
    hls_download(String::from("https://www.dr.dk/"), String::from("test"));
    ()
}
