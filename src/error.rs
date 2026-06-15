use core::fmt;

/// An error returned when a regex pattern fails to compile.
///
/// The error message describes what went wrong (e.g. unclosed group, invalid
/// repetition range). Use the [`Display`](fmt::Display) implementation to
/// present it to users.
///
/// ## Example
///
/// ```
/// use regex::Regex;
///
/// let err = Regex::new(r"(unclosed").unwrap_err();
/// assert!(!err.to_string().is_empty());
/// ```
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Error {
    Syntax(String),
}

impl Error {
    /// Creates a new `Error` with the given message.
    pub(crate) fn new(message: impl Into<String>) -> Self {
        Self::Syntax(message.into())
    }
}

impl fmt::Display for Error {
    /// Formats the error message for display.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Syntax(message) => f.write_str(message),
        }
    }
}

impl std::error::Error for Error {}
