use serde_json::Error as JsonError;
use std::{
    error::Error as StdError,
    fmt::{Display, Error as FmtError, Formatter, Result as FmtResult},
};
use hyper::error::{Error as HyperError};
use reqwest::{
    Error as ReqwestError,
    Response as ReqwestResponse,
    UrlError as ReqwestUrlError,
};

#[derive(Debug)]
pub struct RsgetError {
    details: String,
}

impl RsgetError {
    pub fn new(msg: &str) -> RsgetError {
        RsgetError{details: String::from(msg)}
    }
}

impl Display for RsgetError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        write!(f,"{}",self.details)
    }
}

impl StdError for RsgetError {
    fn description(&self) -> &str {
        &self.details
    }
}

#[derive(Debug)]
pub enum StreamError {
    /// An error that occurred while formatting a string.
    Fmt(FmtError),
    /// An error from the `serde_json` crate while deserializing the body of an
    /// HTTP response.
    Json(JsonError),
    /// An error from the `hyper` crate while performing an HTTP request.
    Hyper(HyperError),
    /// An error from the `reqwest` crate while performing an HTTP request.
    Reqwest(ReqwestError),
    /// An error indicating a bad request when using `reqwest`.
    ReqwestBad(Box<ReqwestResponse>),
    /// An error indicating an invalid request when using `reqwest`.
    ReqwestInvalid(Box<ReqwestResponse>),
    /// An error indicating a parsing issue when using `reqwest`.
    ReqwestParse(ReqwestUrlError),
    /// RsgetError
    Rsget(RsgetError),
}

impl Display for StreamError {
    fn fmt(&self, f: &mut Formatter) -> FmtResult {
        f.write_str(self.description())
    }
}

impl StdError for StreamError {
    fn description(&self) -> &str {
        match *self {
            StreamError::Fmt(ref inner) => inner.description(),
            StreamError::Hyper(ref inner) => inner.description(),
            StreamError::Json(ref inner) => inner.description(),
            StreamError::Reqwest(ref inner) => inner.description(),
            StreamError::ReqwestBad(_) => "Request bad",
            StreamError::ReqwestInvalid(_) => "Request invalid",
            StreamError::ReqwestParse(ref inner) => inner.description(),
            StreamError::Rsget(ref inner) => inner.description(),
        }
    }
}

impl From<FmtError> for StreamError {
    fn from(err: FmtError) -> Self {
        StreamError::Fmt(err)
    }
}

//impl From<serde_json::Error> for Error

impl From<JsonError> for StreamError {
    fn from(err: JsonError) -> Self {
        StreamError::Json(err)
    }
}

impl From<HyperError> for StreamError {
    fn from(err: HyperError) -> Self {
        StreamError::Hyper(err)
    }
}

impl From<ReqwestError> for StreamError {
    fn from(err: ReqwestError) -> Self {
        StreamError::Reqwest(err)
    }
}

impl From<ReqwestUrlError> for StreamError {
    fn from(err: ReqwestUrlError) -> Self {
        StreamError::ReqwestParse(err)
    }
}

