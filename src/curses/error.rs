use std;
use std::io;

#[derive(Debug)]
pub enum Error {
    ColorCount,
    Io(io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

impl From<io::Error> for Error {
    fn from(err: io::Error) -> Error {
        Error::Io(err)
    }
}
