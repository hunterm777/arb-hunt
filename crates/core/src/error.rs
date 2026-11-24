//! The one error type: a sentence that says what to fix.

use std::fmt;

/// Why a check could not run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// An input is missing or out of range.
    Input(String),
    /// venue-specs has no sourced value for a rule the check needs.
    Rules(String),
}

impl Error {
    pub(crate) fn input(msg: impl Into<String>) -> Error {
        Error::Input(msg.into())
    }

    pub(crate) fn rules(msg: impl Into<String>) -> Error {
        Error::Rules(msg.into())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Input(m) | Error::Rules(m) => f.write_str(m),
        }
    }
}

impl std::error::Error for Error {}

/// Result alias for the crate.
pub type Result<T> = std::result::Result<T, Error>;
