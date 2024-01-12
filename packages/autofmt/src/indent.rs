#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum IndentType {
    Spaces,
    Tabs,
}

#[derive(Debug, Clone)]
pub struct IndentOptions {
    width: usize,
    indent_string: String,
    split_line_attributes: bool,
}

impl IndentOptions {
    pub fn new(typ: IndentType, width: usize, split_line_attributes: bool) -> Self {
        assert_ne!(width, 0, "Cannot have an indent width of 0");
        Self {
            width,
            indent_string: match typ {
                IndentType::Tabs => "\t".into(),
                IndentType::Spaces => " ".repeat(width),
            },
            split_line_attributes,
        }
    }

    /// Gets a string containing one indent worth of whitespace
    pub fn indent_str(&self) -> &str {
        &self.indent_string
    }

    /// Computes the line length in characters, counting tabs as the indent width.
    pub fn line_length(&self, line: &str) -> usize {
        line.chars()
            .map(|ch| if ch == '\t' { self.width } else { 1 })
            .sum()
    }

    /// Estimates how many times the line has been indented.
    pub fn count_indents(&self, mut line: &str) -> usize {
        let mut indent = 0;
        while !line.is_empty() {
            // Try to count tabs
            let num_tabs = line.chars().take_while(|ch| *ch == '\t').count();
            if num_tabs > 0 {
                indent += num_tabs;
                line = &line[num_tabs..];
                continue;
            }

            // Try to count spaces
            let num_spaces = line.chars().take_while(|ch| *ch == ' ').count();
            if num_spaces >= self.width {
                // Intentionally floor here to take only the amount of space that matches an indent
                let num_space_indents = num_spaces / self.width;
                indent += num_space_indents;
                line = &line[num_space_indents * self.width..];
                continue;
            }

            // Line starts with either non-indent characters or an unevent amount of spaces,
            // so no more indent remains.
            break;
        }
        indent
    }

    pub fn split_line_attributes(&self) -> bool {
        self.split_line_attributes
    }
}

impl Default for IndentOptions {
    fn default() -> Self {
        Self::new(IndentType::Spaces, 4, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn count_indents() {
        assert_eq!(
            IndentOptions::new(IndentType::Spaces, 4, false).count_indents("no indentation here!"),
            0
        );
        assert_eq!(
            IndentOptions::new(IndentType::Spaces, 4, false).count_indents("    v += 2"),
            1
        );
        assert_eq!(
            IndentOptions::new(IndentType::Spaces, 4, false).count_indents("        v += 2"),
            2
        );
        assert_eq!(
            IndentOptions::new(IndentType::Spaces, 4, false).count_indents("          v += 2"),
            2
        );
        assert_eq!(
            IndentOptions::new(IndentType::Spaces, 4, false).count_indents("\t\tv += 2"),
            2
        );
        assert_eq!(
            IndentOptions::new(IndentType::Spaces, 4, false).count_indents("\t\t  v += 2"),
            2
        );
        assert_eq!(
            IndentOptions::new(IndentType::Spaces, 2, false).count_indents("    v += 2"),
            2
        );
    }
}
