use fast_reg::Regex;

fn main() {
    let re = Regex::new(r"\d+").unwrap();

    for m in re.find_iter("a1 b22 c333") {
        println!("{} at {:?}", m.as_str(), m.range());
    }
}
