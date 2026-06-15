use regex::{Regex, RegexBuilder, RegexSet, RegexSetBuilder};

#[test]
fn literal_and_dot() {
    let re = Regex::new("h.llo").unwrap();
    assert!(re.is_match("well hello there"));
    assert!(!re.is_match("hell\no"));
}

#[test]
fn anchors() {
    assert!(Regex::new("^abc$").unwrap().is_match("abc"));
    assert!(!Regex::new("^abc$").unwrap().is_match("xabc"));
    assert!(!Regex::new("^abc$").unwrap().is_match("abcx"));
}

#[test]
fn alternation_and_groups() {
    let re = Regex::new("(cat|dog)s?").unwrap();
    let caps = re.captures("dogs").unwrap();
    assert_eq!(&caps[0], "dogs");
    assert_eq!(&caps[1], "dog");
}

#[test]
fn repetitions() {
    assert_eq!(
        Regex::new("ab*c")
            .unwrap()
            .find("xxabbbc")
            .unwrap()
            .as_str(),
        "abbbc"
    );
    assert_eq!(
        Regex::new("ab+c").unwrap().find("xxabbc").unwrap().as_str(),
        "abbc"
    );
    assert_eq!(
        Regex::new("ab{2,3}c")
            .unwrap()
            .find("abbbc")
            .unwrap()
            .as_str(),
        "abbbc"
    );
    assert_eq!(
        Regex::new("ab{2}c").unwrap().find("abbc").unwrap().as_str(),
        "abbc"
    );
    assert_eq!(
        Regex::new("a.*?c")
            .unwrap()
            .find("a123c456c")
            .unwrap()
            .as_str(),
        "a123c"
    );
}

#[test]
fn character_classes() {
    assert!(Regex::new(r"^[a-z]+\d\w\s$").unwrap().is_match("abc1x "));
    assert!(Regex::new(r"[^0-9]+").unwrap().is_match("abc"));
    assert!(!Regex::new(r"^[^0-9]+$").unwrap().is_match("abc1"));
}

#[test]
fn case_insensitive_flag() {
    assert!(Regex::new(r"(?i)error").unwrap().is_match("ERROR"));
    assert!(Regex::new(r"(?i)[a-z]+").unwrap().is_match("ABC"));
    assert!(!Regex::new(r"(?i)error").unwrap().is_match("warning"));
}

#[test]
fn extended_group_and_flag_syntax() {
    let re = Regex::new(r"(?:cat|dog)s?").unwrap();
    assert_eq!(re.captures_len(), 1);
    assert!(re.is_match("dogs"));

    assert!(Regex::new(r"a(?i:b)c").unwrap().is_match("aBc"));
    assert!(!Regex::new(r"a(?i:b)c").unwrap().is_match("Abc"));
    assert!(Regex::new(r"(?m)^bar$").unwrap().is_match("foo\nbar\nbaz"));
    assert!(Regex::new(r"(?s)a.c").unwrap().is_match("a\nc"));
    assert!(Regex::new(r"(?x)a b c").unwrap().is_match("abc"));
}

#[test]
fn named_captures_and_replacements() {
    let re = Regex::new(r"(?P<key>\w+)=(?<value>\d+)").unwrap();
    let caps = re.captures("count=42").unwrap();

    assert_eq!(re.captures_len(), 3);
    assert_eq!(
        re.capture_names().collect::<Vec<_>>(),
        [None, Some("key"), Some("value")]
    );
    assert_eq!(caps.name("key").unwrap().as_str(), "count");
    assert_eq!(caps.name("value").unwrap().as_str(), "42");
    assert_eq!(&caps["key"], "count");
    assert_eq!(re.replace("count=42", "${value}:${key}"), "42:count");
}

#[test]
fn escapes_anchors_and_repetition_compatibility() {
    assert!(Regex::new(r"\Aabc\z").unwrap().is_match("abc"));
    assert!(!Regex::new(r"\Aabc\z").unwrap().is_match("xabc"));
    assert!(Regex::new(r"\x41\u0042\u{43}").unwrap().is_match("ABC"));
    assert!(Regex::new(r"^\p{Letter}+$").unwrap().is_match("éA"));
    assert!(Regex::new(r"ab{,3}c").unwrap().is_match("ac"));
    assert!(Regex::new(r"ab{,3}c").unwrap().is_match("abbbc"));
}

#[test]
fn builder_crlf_anchors() {
    let re = RegexBuilder::new(r"(?m)^bar$").crlf(true).build().unwrap();

    assert!(re.is_match("foo\r\nbar\r\nbaz"));
    assert!(!re.is_match("foo\r\n\rbar\r\nbaz"));
}

#[test]
fn builder_syntax_options_are_enforced() {
    assert!(RegexBuilder::new(r"\141").build().is_err());
    assert!(
        RegexBuilder::new(r"\141")
            .octal(true)
            .build()
            .unwrap()
            .is_match("a")
    );

    assert!(RegexBuilder::new(r"\u{61}").unicode(false).build().is_err());
    assert!(
        RegexBuilder::new(r"\p{Letter}")
            .unicode(false)
            .build()
            .is_err()
    );
}

#[test]
fn builder_compile_limits_are_enforced() {
    assert!(RegexBuilder::new("(a)").nest_limit(0).build().is_err());
    assert!(RegexBuilder::new("abcdef").size_limit(2).build().is_err());
    assert!(RegexBuilder::new("abcdef").size_limit(20).build().is_ok());
}

#[test]
fn fast_pattern_counts() {
    assert_eq!(Regex::new(r"\d+").unwrap().find_iter("a12 b345").count(), 2);
    assert_eq!(Regex::new(r"\w+").unwrap().find_iter("a12 b_3!").count(), 2);
    assert_eq!(
        Regex::new(r"[a-zA-Z_]+")
            .unwrap()
            .find_iter("a12 b_3!")
            .count(),
        2
    );
    assert_eq!(
        Regex::new(r"\d{4}").unwrap().find_iter("123456789").count(),
        2
    );
    assert_eq!(
        Regex::new(r"\w{2,}").unwrap().find_iter("a ab cde").count(),
        2
    );
    assert_eq!(
        Regex::new(r"(?i)error")
            .unwrap()
            .find_iter("error ERROR Error warning")
            .count(),
        3
    );
    assert_eq!(
        Regex::new(r"(a|aa)+b")
            .unwrap()
            .find_iter("aaab aab")
            .count(),
        2
    );
    assert_eq!(
        Regex::new(r"(a+)+b").unwrap().find_iter("aaab aab").count(),
        2
    );
}

#[test]
fn word_boundaries() {
    assert!(Regex::new(r"\bcat\b").unwrap().is_match("a cat!"));
    assert!(!Regex::new(r"\bcat\b").unwrap().is_match("scatter"));
    assert!(Regex::new(r"\Bcat\B").unwrap().is_match("scatterx"));
}

#[test]
fn find_iter_handles_empty_matches() {
    let found: Vec<_> = Regex::new("a*")
        .unwrap()
        .find_iter("bbb")
        .map(|m| m.as_str().to_string())
        .collect();
    assert_eq!(found, vec!["", "", "", ""]);
}

#[test]
fn captures_iter() {
    let found: Vec<_> = Regex::new(r"(\d+)")
        .unwrap()
        .captures_iter("a1 b22")
        .map(|c| c[1].to_string())
        .collect();
    assert_eq!(found, vec!["1", "22"]);
}

#[test]
fn split_and_replace() {
    let re = Regex::new(r",\s*").unwrap();
    let parts: Vec<_> = re.split("a, b,c").collect();
    assert_eq!(parts, vec!["a", "b", "c"]);

    let re = Regex::new(r"(\w+)=(\d+)").unwrap();
    assert_eq!(re.replace("x=12 y=3", "$2:$1"), "12:x y=3");
    assert_eq!(re.replace_all("x=12 y=3", "$2:$1"), "12:x 3:y");
}

#[test]
fn unicode_offsets_are_valid() {
    let re = Regex::new("é+").unwrap();
    let m = re.find("aééz").unwrap();
    assert_eq!(m.start(), 1);
    assert_eq!(m.end(), 5);
    assert_eq!(m.as_str(), "éé");
}

#[test]
fn parse_errors() {
    assert!(Regex::new("(").is_err());
    assert!(Regex::new("[").is_err());
    assert!(Regex::new("a{3,2}").is_err());
    assert!(Regex::new("*").is_err());
}

#[test]
fn regex_set_reports_matching_pattern_indexes() {
    let set = RegexSet::new([r"\d+", r"\w+", r"^foo", r"bar$"]).unwrap();
    let matches = set.matches("foo 123");

    assert_eq!(set.len(), 4);
    assert_eq!(set.patterns(), &[r"\d+", r"\w+", r"^foo", r"bar$"]);
    assert!(set.is_match("foo 123"));
    assert!(!set.is_match("!!!"));
    assert!(matches.matched_any());
    assert!(matches.matched(0));
    assert!(matches.matched(1));
    assert!(matches.matched(2));
    assert!(!matches.matched(3));
    assert!(!matches.matched(99));
    assert_eq!(matches.iter().collect::<Vec<_>>(), vec![0, 1, 2]);
    assert_eq!((&matches).into_iter().collect::<Vec<_>>(), vec![0, 1, 2]);
    assert_eq!(matches.into_iter().collect::<Vec<_>>(), vec![0, 1, 2]);
}

#[test]
fn regex_set_builder_flags_apply_to_patterns() {
    let set = RegexSetBuilder::new([r"^bar$", r"a.c", r"a b"])
        .multi_line(true)
        .dot_matches_new_line(true)
        .ignore_whitespace(true)
        .build()
        .unwrap();

    assert!(set.matches("foo\nbar\nbaz").matched(0));
    assert!(set.matches("a\nc").matched(1));
    assert!(set.matches("ab").matched(2));
}

#[test]
fn regex_set_builder_extra_options_apply_to_patterns() {
    let set = RegexSetBuilder::new([r"(?m)^bar$", r"\141"])
        .crlf(true)
        .octal(true)
        .build()
        .unwrap();

    assert!(set.matches("foo\r\nbar\r\na").matched(0));
    assert!(set.matches("foo\r\nbar\r\na").matched(1));
    assert!(
        RegexSetBuilder::new([r"(a)"])
            .nest_limit(0)
            .build()
            .is_err()
    );
    assert!(
        RegexSetBuilder::new([r"abcdef"])
            .size_limit(2)
            .build()
            .is_err()
    );
}
