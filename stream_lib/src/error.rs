use std::io::Error as IoError;
use reqwest::Error as ReqwestError;
use hls_m3u8::Error as HlsError;
use url::ParseError;


pub enum Error {
    /// M3U8 error
    Hls(HlsError),
    /// Http error.
    Reqwest(ReqwestError),
    /// Io error.
    Io(IoError),
    /// Url error.
    Url(ParseError),
}

impl From<HlsError> for Error {
    fn from(err: HlsError) -> Self {
        Error::Hls(err)
    }
}

impl From<ReqwestError> for Error {
    fn from(err: ReqwestError) -> Self {
        Error::Reqwest(err)
    }
}

impl From<IoError> for Error {
    fn from(err: IoError) -> Self {
        Error::Io(err)
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        Error::Url(err)
    }
}
