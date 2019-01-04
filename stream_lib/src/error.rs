use std::{
    error::Error as StdError,
    fmt::{Display, Formatter, Result as FmtResult},
    io::Error as IoError,
};
use reqwest::Error as ReqwestError;
use hls_m3u8::Error as HlsError;
use url::ParseError;

#[derive(Debug)]
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

impl Display for Error {
    fn fmt(&self, f: &mut Formatter) -> FmtResult { f.write_str(self.description()) }
}

impl StdError for Error {
    fn description(&self) -> &str {
        use self::Error::*;

        match *self {
            Hls(ref inner) => inner.description(),
            Reqwest(ref inner) => inner.description(),
            Io(ref inner) => inner.description(),
            Url(ref inner) => inner.description(),
        }
    }
}
