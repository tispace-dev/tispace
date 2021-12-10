use std::fmt;

pub type GenericError = Box<dyn std::error::Error + Send + Sync>;
pub type Result<T> = std::result::Result<T, GenericError>;

#[derive(Debug)]
pub struct Error {
    detail: String,
}

impl Error {
    pub fn new(detail: &str) -> Self {
        Error {
            detail: detail.to_string(),
        }
    }
}

impl std::error::Error for Error {}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.detail)
    }
}
