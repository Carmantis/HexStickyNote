use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Error, Serialize)]
#[serde(tag = "kind", content = "message")]
pub enum CalError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("IO error: {0}")]
    Io(String),

    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl From<rusqlite::Error> for CalError {
    fn from(e: rusqlite::Error) -> Self {
        CalError::Database(e.to_string())
    }
}

impl From<serde_json::Error> for CalError {
    fn from(e: serde_json::Error) -> Self {
        CalError::Serialization(e.to_string())
    }
}

impl From<std::io::Error> for CalError {
    fn from(e: std::io::Error) -> Self {
        CalError::Io(e.to_string())
    }
}
