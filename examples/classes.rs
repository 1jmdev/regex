use fast_reg::Regex;

fn main() {
    let re = Regex::new(r"^[a-z]+\d\w\s$").unwrap();

    println!("{}", re.is_match("abc1x "));
    println!("{}", re.is_match("abcxx "));
}
