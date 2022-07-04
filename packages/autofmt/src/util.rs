pub fn find_bracket_end(contents: &str) -> Option<usize> {
    let mut depth = 0;

    for (i, c) in contents.chars().enumerate() {
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;

            if depth == 0 {
                return Some(i);
            }
        }
    }

    None
}
