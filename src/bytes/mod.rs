mod api;
mod matcher;

pub use crate::regexset::bytes::{RegexSet, RegexSetBuilder, SetMatches, SetMatchesIter};
pub use api::{
    CaptureMatches, CaptureNames, Captures, FindMatches, Match, Regex, RegexBuilder, Replacer,
    Split, SplitN,
};
