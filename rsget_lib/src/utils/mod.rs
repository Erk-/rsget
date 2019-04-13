pub mod downloaders;
pub mod sites;
pub mod error;
//pub mod stream;

use stream_lib::StreamType;

#[allow(unreachable_patterns)]
/// Utility to get a url from a `StreamType`.
pub fn stream_type_to_url(stream: StreamType) -> String {
    match stream {
        StreamType::Chuncked(req) => req.url().to_string(),
        StreamType::HLS(req) => req.url().to_string(),
        StreamType::NamedPlaylist(req, _) => req.url().to_string(),
        _ => unimplemented!(),
    }
}
