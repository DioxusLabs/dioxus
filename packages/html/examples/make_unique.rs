use std::collections::HashSet;

fn main() {
    let f = include_str!("./unique");

    let mut names = HashSet::new();

    for line in f.lines() {
        let line = line.trim();
        if line.starts_with("//") {
            continue;
        }
        let mut split = line.split_terminator(':');
        let (left, right) = (split.next().unwrap(), split.next().unwrap());
        names.insert(left);
        // dbg!(left, right);
    }

    for name in names {
        println!("{}", name);
    }
}
