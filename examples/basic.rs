use regex::Regex;

fn main() {
    let re = Regex::new(r"h.llo").unwrap();

    println!("{}", re.is_match("well hello there"));

    if let Some(m) = re.find("well hello there") {
        println!("{} {} {}", m.as_str(), m.start(), m.end());
    }
}
