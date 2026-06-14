use regex::Regex;

fn main() {
    let re = Regex::new(r"(\w+)=(\d+)").unwrap();
    let caps = re.captures("name=42").unwrap();

    println!("full: {}", &caps[0]);
    println!("key: {}", &caps[1]);
    println!("value: {}", &caps[2]);
}
