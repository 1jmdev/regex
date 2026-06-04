mod api;
mod arch;
pub(crate) mod fast;
pub(crate) mod slots;

pub use crate::regexset::string::{RegexSet, SetMatches, SetMatchesIter};
pub use api::{CaptureMatches, Captures, FindMatches, Match, Regex, Replacer, Split};
