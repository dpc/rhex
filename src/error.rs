
use std::sync::{mpsc};
use std::error::FromError;

/// Possible errors that could terminate Controller
pub enum Error<T> {
    Receive(mpsc::RecvError),
    Transmit(mpsc::SendError<T>),
}

impl<T> FromError<mpsc::RecvError> for Error<T> {
    fn from_error(err: mpsc::RecvError) -> Error<T> {
        Error::Receive(err)
    }
}

impl<T> FromError<mpsc::SendError<T>> for Error<T> {
    fn from_error(err: mpsc::SendError<T>) -> Error<T> {
        Error::Transmit(err)
    }
}
