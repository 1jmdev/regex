mod api;
mod arch;
pub(crate) mod fast;
pub(crate) mod slots;

pub use crate::regexset::string::{RegexSet, RegexSetBuilder, SetMatches, SetMatchesIter};
pub use api::{
    CaptureMatches, CaptureNames, Captures, FindMatches, Match, Regex, RegexBuilder, Replacer,
    Split, SplitN,
};
