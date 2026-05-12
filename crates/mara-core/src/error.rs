//! Core error type for the Mara pipeline.
//!
//! Per ADR-0006, public APIs in library crates return concrete
//! error enums derived with `thiserror`.  Errors are
//! `#[non_exhaustive]` so additive variants do not break SemVer.

use std::io;

use thiserror::Error;

/// Convenience alias for `Result<T, Error>`.
pub type Result<T, E = Error> = std::result::Result<T, E>;

/// Errors produced by `mara-core` orchestration.
#[derive(Debug, Error)]
#[non_exhaustive]
pub enum Error {
    /// Underlying I/O error.
    #[error("I/O error at {path:?}: {source}")]
    Io {
        /// Filesystem path the operation targeted, if known.
        path: Option<String>,
        /// Underlying I/O error.
        #[source]
        source: io::Error,
    },

    /// Configuration parsing or validation failed.
    #[error("configuration error: {message}")]
    Config {
        /// Operator-facing description of the configuration problem.
        message: String,
        /// Source file path, if known.
        path: Option<String>,
    },

    /// Adapter operation failed.
    #[error("adapter '{adapter}': {message}")]
    Adapter {
        /// Adapter logical name.
        adapter: String,
        /// Operator-facing description.
        message: String,
    },

    /// Sink operation failed.
    #[error("sink '{sink}': {message}")]
    Sink {
        /// Sink logical name.
        sink: String,
        /// Operator-facing description.
        message: String,
    },

    /// Policy stage failed or trapped.
    #[error("policy stage '{stage}': {message}")]
    Policy {
        /// Stage logical name (matches the policy chain config).
        stage: String,
        /// Operator-facing description.
        message: String,
    },

    /// WAL operation failed.
    #[error("WAL error: {0}")]
    Wal(String),

    /// Pipeline shutdown was requested.
    #[error("pipeline is shutting down")]
    Shutdown,

    /// Internal error that does not fit other variants.
    /// Should only be used as a last resort.
    #[error("internal error: {0}")]
    Internal(String),
}

impl From<io::Error> for Error {
    fn from(source: io::Error) -> Self {
        Self::Io { path: None, source }
    }
}
