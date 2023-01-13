pub fn find_bracket_end(contents: &str) -> Option<usize> {
    let mut depth = 0;

    let mut len = 0;

    for c in contents.chars() {
        len += c.len_utf8();
        if c == '{' {
            depth += 1;
        } else if c == '}' {
            depth -= 1;

            if depth == 0 {
                return Some(len);
            }
        }
    }

    None
}
