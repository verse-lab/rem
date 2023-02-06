use std::string::FromUtf8Error;

#[derive(Debug)]
pub enum Error {
    IO(std::io::Error),
    StringFormat(FromUtf8Error),
    TypeError(crate::typ::Error),
    Other(String),
}
impl From<Error> for String {
    fn from(val: Error) -> Self {
        match val {
            Error::IO(ioe) => format!("IO({:?})", ioe),
            Error::TypeError(e) => format!("TypeError({:?})", e),
            Error::StringFormat(f) => format!("{:?}", f),
            Error::Other(st) => format!("Other Error: {}", st),
        }
    }
}

impl From<crate::typ::Error> for Error {
    fn from(val: crate::typ::Error) -> Self {
        Error::TypeError(val)
    }
}

impl From<std::io::Error> for Error {
    fn from(v: std::io::Error) -> Self {
        Error::IO(v)
    }
}
impl From<FromUtf8Error> for Error {
    fn from(v: FromUtf8Error) -> Self {
        Error::StringFormat(v)
    }
}
impl From<String> for Error {
    fn from(v: String) -> Self {
        Error::Other(v)
    }
}

impl<'a> From<&'a str> for Error {
    fn from(v: &'a str) -> Self {
        Error::Other(v.into())
    }
}

impl From<!> for Error {
    fn from(v: !) -> Self {
        match v {}
    }
}
