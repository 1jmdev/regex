use regex::bytes::{Regex, RegexBuilder, RegexSet, RegexSetBuilder};

fn assert_same(pattern: &str, haystack: &[u8]) {
    let fast = Regex::new(pattern).unwrap();
    let official = rust_regex::bytes::Regex::new(pattern).unwrap();

    assert_eq!(
        fast.is_match(haystack),
        official.is_match(haystack),
        "is_match {pattern}"
    );
    assert_eq!(
        fast.find(haystack).map(|m| m.range()),
        official.find(haystack).map(|m| m.range()),
        "find {pattern}"
    );
    assert_eq!(
        fast.find_iter(haystack)
            .map(|m| m.range())
            .collect::<Vec<_>>(),
        official
            .find_iter(haystack)
            .map(|m| m.range())
            .collect::<Vec<_>>(),
        "find_iter {pattern}"
    );
}

#[test]
fn bytes_match_invalid_utf8_like_regex_crate() {
    let haystack = b"a\xff12 b\x80\xff345";
    for pattern in [r"\d+", r"\w+", r".", r"[^0-9]+", r"\b\w+\b"] {
        assert_same(pattern, haystack);
    }
}

#[test]
fn bytes_captures_match_regex_crate() {
    let haystack = b"\xff x=12 y=345";
    let fast = Regex::new(r"(\w+)=(\d+)").unwrap();
    let official = rust_regex::bytes::Regex::new(r"(\w+)=(\d+)").unwrap();

    let caps = fast.captures(haystack).unwrap();
    let expected = official.captures(haystack).unwrap();
    for i in 0..caps.len() {
        assert_eq!(
            caps.get(i).map(|m| m.range()),
            expected.get(i).map(|m| m.range())
        );
        assert_eq!(
            caps.get(i).map(|m| m.as_bytes()),
            expected.get(i).map(|m| m.as_bytes())
        );
    }
    assert_eq!(&caps[1], b"x");
    assert_eq!(&caps[2], b"12");
}

#[test]
fn bytes_split_and_replace() {
    let re = Regex::new(r",\s*").unwrap();
    let parts: Vec<_> = re.split(b"a, b,c\xff").collect();
    assert_eq!(parts, vec![&b"a"[..], &b"b"[..], &b"c\xff"[..]]);

    let re = Regex::new(r"(\w+)=(\d+)").unwrap();
    assert_eq!(re.replace(b"x=12 y=3", b"$2:$1").as_ref(), b"12:x y=3");
    assert_eq!(re.replace_all(b"x=12 y=3", b"$2:$1").as_ref(), b"12:x 3:y");
    assert_eq!(
        re.replace_all(b"x=12", |caps: &regex::bytes::Captures<'_>| {
            caps[2].to_vec()
        })
        .as_ref(),
        b"12"
    );
}

#[test]
fn bytes_fast_patterns_match_regex_crate() {
    let haystack = b"a12 b_3 ERROR aaab aab 123456789";
    for pattern in [
        r"\d+",
        r"\w+",
        r"[a-zA-Z_]+",
        r"\d{4}",
        r"\w{2,}",
        r"(?i)error",
        r"a+b",
        r"(a|aa)+b",
        r"(a+)+b",
    ] {
        assert_same(pattern, haystack);
    }
}

#[test]
fn bytes_extended_syntax() {
    let re = Regex::new(r"(?:cat|dog)s?").unwrap();
    assert_eq!(re.captures_len(), 1);
    assert!(re.is_match(b"dogs"));

    assert!(Regex::new(r"a(?i:b)c").unwrap().is_match(b"aBc"));
    assert!(Regex::new(r"(?m)^bar$").unwrap().is_match(b"foo\nbar\nbaz"));
    assert!(Regex::new(r"(?s)a.c").unwrap().is_match(b"a\nc"));
    assert!(Regex::new(r"\Aabc\z").unwrap().is_match(b"abc"));
    assert!(Regex::new(r"\x41\u0042\u{43}").unwrap().is_match(b"ABC"));
    assert!(Regex::new(r"ab{,3}c").unwrap().is_match(b"abbbc"));
}

#[test]
fn bytes_builder_extra_options_are_enforced() {
    let re = RegexBuilder::new(r"(?m)^bar$").crlf(true).build().unwrap();
    assert!(re.is_match(b"foo\r\nbar\r\nbaz"));

    assert!(RegexBuilder::new(r"\141").build().is_err());
    assert!(
        RegexBuilder::new(r"\141")
            .octal(true)
            .build()
            .unwrap()
            .is_match(b"a")
    );
    assert!(RegexBuilder::new(r"\u{61}").unicode(false).build().is_err());
    assert!(RegexBuilder::new(r"(a)").nest_limit(0).build().is_err());
    assert!(RegexBuilder::new(r"abcdef").size_limit(2).build().is_err());
}

#[test]
fn bytes_named_captures_and_replacements() {
    let re = Regex::new(r"(?P<key>\w+)=(?<value>\d+)").unwrap();
    let caps = re.captures(b"count=42").unwrap();

    assert_eq!(
        re.capture_names().collect::<Vec<_>>(),
        [None, Some("key"), Some("value")]
    );
    assert_eq!(caps.name("key").unwrap().as_bytes(), b"count");
    assert_eq!(caps.name("value").unwrap().as_bytes(), b"42");
    assert_eq!(&caps["key"], b"count");
    assert_eq!(
        re.replace(b"count=42", b"${value}:${key}").as_ref(),
        b"42:count"
    );
}

#[test]
fn bytes_utf8_pattern_literals_match_encoded_bytes() {
    let re = Regex::new("é+").unwrap();
    let m = re.find("xéé".as_bytes()).unwrap();
    assert_eq!(m.as_bytes(), "éé".as_bytes());
    assert_eq!(m.range(), 1..5);
}

#[test]
fn bytes_regex_set_reports_matching_pattern_indexes() {
    let set = RegexSet::new([r"\d+", r"\w+", r"^foo", r"bar$"]).unwrap();
    let matches = set.matches(b"foo 123");

    assert_eq!(set.len(), 4);
    assert_eq!(set.patterns(), &[r"\d+", r"\w+", r"^foo", r"bar$"]);
    assert!(set.is_match(b"foo 123"));
    assert!(!set.is_match(b"!!!"));
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
fn bytes_regex_set_builder_flags_apply_to_patterns() {
    let set = RegexSetBuilder::new([r"^bar$", r"a.c", r"a b"])
        .multi_line(true)
        .dot_matches_new_line(true)
        .ignore_whitespace(true)
        .build()
        .unwrap();

    assert!(set.matches(b"foo\nbar\nbaz").matched(0));
    assert!(set.matches(b"a\nc").matched(1));
    assert!(set.matches(b"ab").matched(2));
}

#[test]
fn bytes_regex_set_builder_extra_options_apply_to_patterns() {
    let set = RegexSetBuilder::new([r"(?m)^bar$", r"\141"])
        .crlf(true)
        .octal(true)
        .build()
        .unwrap();

    assert!(set.matches(b"foo\r\nbar\r\na").matched(0));
    assert!(set.matches(b"foo\r\nbar\r\na").matched(1));
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
