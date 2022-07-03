use std::{error::Error, fmt::Display};

#[derive(Debug)]
pub struct NoneError;

impl Display for NoneError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NoneError")
    }
}

impl Error for NoneError {}

pub trait OptionIntoResult<T> {
    fn into_result(self) -> Result<T, Box<NoneError>>;
}

impl<T> OptionIntoResult<T> for Option<T> {
    fn into_result(self) -> Result<T, Box<NoneError>> {
        self.ok_or(Box::new(NoneError))
    }
}
