use rpm;
use thiserror::Error;
use toml;

use std::io;
use std::path;
use std::process;

use crate::bom;

/// ChiriError models unsuccessful program states.
#[derive(Error, Debug)]
pub enum ChiriError {
    #[error("{0}")]
    ValidationError(String),

    #[error("unable to render path: {0}")]
    UnrenderablePathError(path::PathBuf),

    #[error("unsupported file system file type: {0}")]
    UnsupportedFileSystemFileTypeError(path::PathBuf),

    #[error("copy error: {source}: origin: {origin}, destination: {destination}")]
    CopyError {
        source: io::Error,
        origin: path::PathBuf,
        destination: path::PathBuf,
    },

    #[error("{source}: {path}")]
    IOError {
        source: io::Error,
        path: path::PathBuf,
    },

    #[error("unable to lookup parent of path: {0}")]
    ParentLookupError(path::PathBuf),

    #[error(transparent)]
    StripPrefixError(#[from] path::StripPrefixError),

    #[error(transparent)]
    TOMLParseError(#[from] toml::de::Error),

    #[error(transparent)]
    LibarchiveError(#[from] libarchive2::Error),

    #[error(transparent)]
    WalkdirError(#[from] walkdir::Error),

    #[error(transparent)]
    UreqError(#[from] ureq::Error),

    #[error(transparent)]
    AppImageError(#[from] appimage::Error),

    #[error(transparent)]
    RPMError(#[from] rpm::Error),

    #[error(transparent)]
    BomError(#[from] bom::Error),
}

pub trait ResultExt<T> {
    // with_path wraps errors, preserving crucial path details.
    fn with_path<P: AsRef<path::Path>>(self, path: P) -> Result<T, ChiriError>;
}

impl<T> ResultExt<T> for std::io::Result<T> {
    fn with_path<P: AsRef<path::Path>>(self, path: P) -> Result<T, ChiriError> {
        self.map_err(|source| ChiriError::IOError {
            source,
            path: path.as_ref().to_path_buf(),
        })
    }
}

impl die::PrintExit for ChiriError {
    fn print_exit(&self) -> ! {
        eprintln!("{}", self);
        process::exit(die::DEFAULT_EXIT_CODE);
    }
}
