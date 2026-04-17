use dioxus_autofmt::write_block_out;
use dioxus_rsx::CallBody;
use syn::parse::Parser;

use super::serialize::normalize_formatted_rsx;

pub(super) fn normalize_debug_rsx(raw: &str) -> String {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return trimmed.to_string();
    }

    CallBody::parse_strict
        .parse_str(trimmed)
        .ok()
        .and_then(|body| write_block_out(&body).map(normalize_formatted_rsx))
        .unwrap_or_else(|| trimmed.to_string())
}

pub(super) fn unified_rsx_diff(expected: &str, actual: &str) -> String {
    let expected_lines: Vec<&str> = expected.lines().collect();
    let actual_lines: Vec<&str> = actual.lines().collect();
    let mut lcs = vec![vec![0usize; actual_lines.len() + 1]; expected_lines.len() + 1];

    for i in (0..expected_lines.len()).rev() {
        for j in (0..actual_lines.len()).rev() {
            lcs[i][j] = if expected_lines[i] == actual_lines[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }

    let mut i = 0;
    let mut j = 0;
    let mut diff_lines = Vec::new();

    while i < expected_lines.len() && j < actual_lines.len() {
        if expected_lines[i] == actual_lines[j] {
            diff_lines.push(format!(" {}", expected_lines[i]));
            i += 1;
            j += 1;
        } else if lcs[i + 1][j] >= lcs[i][j + 1] {
            diff_lines.push(format!("-{}", expected_lines[i]));
            i += 1;
        } else {
            diff_lines.push(format!("+{}", actual_lines[j]));
            j += 1;
        }
    }

    while i < expected_lines.len() {
        diff_lines.push(format!("-{}", expected_lines[i]));
        i += 1;
    }

    while j < actual_lines.len() {
        diff_lines.push(format!("+{}", actual_lines[j]));
        j += 1;
    }

    format!("--- expected\n+++ actual\n@@\n{}", diff_lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::unified_rsx_diff;

    #[test]
    fn diff_preserves_equal_lines() {
        assert_eq!(
            unified_rsx_diff("a\nb", "a\nb"),
            "--- expected\n+++ actual\n@@\n a\n b"
        );
    }

    #[test]
    fn diff_marks_insertions() {
        assert_eq!(
            unified_rsx_diff("a", "a\nb"),
            "--- expected\n+++ actual\n@@\n a\n+b"
        );
    }

    #[test]
    fn diff_marks_deletions() {
        assert_eq!(
            unified_rsx_diff("a\nb", "a"),
            "--- expected\n+++ actual\n@@\n a\n-b"
        );
    }

    #[test]
    fn diff_marks_mixed_edits() {
        assert_eq!(
            unified_rsx_diff("a\nb\nc", "a\nx\nc"),
            "--- expected\n+++ actual\n@@\n a\n-b\n+x\n c"
        );
    }
}
