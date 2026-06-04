use fast_reg::Regex;

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
