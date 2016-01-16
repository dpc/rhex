use std::io;
use std;

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}
