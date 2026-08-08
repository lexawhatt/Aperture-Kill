use std::io;

use thiserror::Error;

pub type PackageResult<T> = Result<T, LevelPackageError>;

#[derive(Debug, Error)]
pub enum LevelPackageError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("invalid package data: {0}")]
    InvalidData(String),
    #[error("unsupported package schema {schema}")]
    UnsupportedSchema { schema: u8 },
    #[error("unsupported package entry {entry:?}")]
    UnsupportedEntry { entry: String },
    #[error("unsupported world encoding {encoding:?}")]
    UnsupportedEncoding { encoding: String },
    #[error("chunk {chunk_id} byte size mismatch")]
    ChunkSizeMismatch { chunk_id: u32 },
    #[error("chunk {chunk_id} sha256 mismatch")]
    ChunkChecksumMismatch { chunk_id: u32 },
    #[error("chunk {chunk_id} header does not match world.index")]
    ChunkHeaderMismatch { chunk_id: u32 },
    #[error("chunk {chunk_id} object counts do not match world.index")]
    ChunkCountsMismatch { chunk_id: u32 },
    #[error("invalid package path {path:?}")]
    InvalidPackagePath { path: String },
    #[error("package entry {path} is a symlink")]
    SymlinkPackageEntry { path: String },
    #[error("quantized coordinate overflow for {name}")]
    CoordinateOverflow { name: String },
    #[error("{name} does not fit {storage}")]
    CoordinateOutOfRange { name: String, storage: &'static str },
}

impl From<LevelPackageError> for io::Error {
    fn from(error: LevelPackageError) -> Self {
        match error {
            LevelPackageError::Io(error) => error,
            other => io::Error::new(io::ErrorKind::InvalidData, other),
        }
    }
}

pub fn invalid_data(message: impl Into<String>) -> LevelPackageError {
    LevelPackageError::InvalidData(message.into())
}
