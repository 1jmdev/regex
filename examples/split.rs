use regex::Regex;

fn main() {
    let re = Regex::new(r",\s*").unwrap();

    for part in re.split("red, green,blue") {
        println!("{}", part);
    }
}
