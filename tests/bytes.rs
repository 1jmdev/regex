use fast_reg::bytes::Regex;

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
    assert_eq!(re.replace(b"x=12 y=3", b"$2:$1"), b"12:x y=3");
    assert_eq!(re.replace_all(b"x=12 y=3", b"$2:$1"), b"12:x 3:y");
    assert_eq!(
        re.replace_all(b"x=12", |caps: &fast_reg::bytes::Captures<'_>| {
            caps[2].to_vec()
        }),
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
fn bytes_utf8_pattern_literals_match_encoded_bytes() {
    let re = Regex::new("é+").unwrap();
    let m = re.find("xéé".as_bytes()).unwrap();
    assert_eq!(m.as_bytes(), "éé".as_bytes());
    assert_eq!(m.range(), 1..5);
}
