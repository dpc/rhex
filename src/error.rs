
use std::sync::{mpsc};
use std::convert::From;

/// Possible errors that could terminate Controller
pub enum Error<T> {
    Receive(mpsc::RecvError),
    Transmit(mpsc::SendError<T>),
}

impl<T> From<mpsc::RecvError> for Error<T> {
    fn from(err: mpsc::RecvError) -> Error<T> {
        Error::Receive(err)
    }
}

impl<T> From<mpsc::SendError<T>> for Error<T> {
    fn from(err: mpsc::SendError<T>) -> Error<T> {
        Error::Transmit(err)
    }
}
