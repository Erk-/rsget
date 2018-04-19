use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct StreamError {
    details: String,
}

impl StreamError {
    pub fn new(msg: &str) -> StreamError {
        StreamError{details: String::from(msg)}
    }
}

impl fmt::Display for StreamError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f,"{}",self.details)
    }
}

impl Error for StreamError {
    fn description(&self) -> &str {
        &self.details
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn raise_stream_error(yes: bool) -> Result<(), StreamError> {
        if yes {
            Err(StreamError::new("bork bork"))
        } else {
            Ok(())
        }
    }
}
        
        
