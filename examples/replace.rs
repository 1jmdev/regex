use fast_reg::Regex;

fn main() {
    let re = Regex::new(r"(\w+)=(\d+)").unwrap();

    let one = re.replace("x=12 y=3", "$2:$1");
    let all = re.replace_all("x=12 y=3", "$2:$1");

    println!("{}", one);
    println!("{}", all);
}
