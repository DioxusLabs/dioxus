use owo_colors::{
    colors::{css::LightBlue, BrightRed},
    OwoColorize, Stream,
};
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use crate::metadata::{
    AnyLoopInfo, ClosureInfo, ConditionalInfo, ForInfo, HookInfo, IfInfo, MatchInfo, WhileInfo,
};

/// The result of checking a Dioxus file for issues.
pub struct IssueReport {
    pub path: PathBuf,
    pub crate_root: PathBuf,
    pub file_content: String,
    pub issues: Vec<Issue>,
}

impl IssueReport {
    pub fn new<S: ToString>(
        path: PathBuf,
        crate_root: PathBuf,
        file_content: S,
        issues: Vec<Issue>,
    ) -> Self {
        Self {
            path,
            crate_root,
            file_content: file_content.to_string(),
            issues,
        }
    }
}

fn lightblue(text: &str) -> String {
    text.if_supports_color(Stream::Stderr, |text| text.fg::<LightBlue>())
        .to_string()
}

fn brightred(text: &str) -> String {
    text.if_supports_color(Stream::Stderr, |text| text.fg::<BrightRed>())
        .to_string()
}

fn bold(text: &str) -> String {
    text.if_supports_color(Stream::Stderr, |text| text.bold())
        .to_string()
}

impl Display for IssueReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let relative_file = Path::new(&self.path)
            .strip_prefix(&self.crate_root)
            .unwrap_or(Path::new(&self.path))
            .display();

        let pipe_char = lightblue("|");

        for (i, issue) in self.issues.iter().enumerate() {
            let hook_info = issue.hook_info();
            let hook_span = hook_info.span;
            let hook_name_span = hook_info.name_span;
            let error_line = format!("{}: {}", brightred("error"), issue);
            writeln!(f, "{}", bold(&error_line))?;
            writeln!(
                f,
                "  {} {}:{}:{}",
                lightblue("-->"),
                relative_file,
                hook_span.start.line,
                hook_span.start.column + 1
            )?;
            let max_line_num_len = hook_span.end.line.to_string().len();
            writeln!(f, "{:>max_line_num_len$} {}", "", pipe_char)?;
            for (i, line) in self.file_content.lines().enumerate() {
                let line_num = i + 1;
                if line_num >= hook_span.start.line && line_num <= hook_span.end.line {
                    writeln!(
                        f,
                        "{:>max_line_num_len$} {} {}",
                        lightblue(&line_num.to_string()),
                        pipe_char,
                        line,
                    )?;
                    if line_num == hook_span.start.line {
                        let mut caret = String::new();
                        for _ in 0..hook_name_span.start.column {
                            caret.push(' ');
                        }
                        for _ in hook_name_span.start.column..hook_name_span.end.column {
                            caret.push('^');
                        }
                        writeln!(
                            f,
                            "{:>max_line_num_len$} {} {}",
                            "",
                            pipe_char,
                            brightred(&caret),
                        )?;
                    }
                }
            }

            let note_text_prefix = format!(
                "{:>max_line_num_len$} {}\n{:>max_line_num_len$} {} note:",
                "",
                pipe_char,
                "",
                lightblue("=")
            );

            match issue {
                Issue::HookInsideConditional(
                    _,
                    ConditionalInfo::If(IfInfo { span: _, head_span }),
                )
                | Issue::HookInsideConditional(
                    _,
                    ConditionalInfo::Match(MatchInfo { span: _, head_span }),
                ) => {
                    if let Some(source_text) = &head_span.source_text {
                        writeln!(
                            f,
                            "{} `{} {{ … }}` is the conditional",
                            note_text_prefix, source_text,
                        )?;
                    }
                }
                Issue::HookInsideLoop(_, AnyLoopInfo::For(ForInfo { span: _, head_span }))
                | Issue::HookInsideLoop(_, AnyLoopInfo::While(WhileInfo { span: _, head_span })) => {
                    if let Some(source_text) = &head_span.source_text {
                        writeln!(
                            f,
                            "{} `{} {{ … }}` is the loop",
                            note_text_prefix, source_text,
                        )?;
                    }
                }
                Issue::HookInsideLoop(_, AnyLoopInfo::Loop(_)) => {
                    writeln!(f, "{} `loop {{ … }}` is the loop", note_text_prefix,)?;
                }
                Issue::HookOutsideComponent(_) | Issue::HookInsideClosure(_, _) => {}
            }

            if i < self.issues.len() - 1 {
                writeln!(f)?;
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(clippy::enum_variant_names)] // we'll add non-hook ones in the future
/// Issues that might be found via static analysis of a Dioxus file.
pub enum Issue {
    /// https://dioxuslabs.com/learn/0.4/reference/hooks#no-hooks-in-conditionals
    HookInsideConditional(HookInfo, ConditionalInfo),
    /// https://dioxuslabs.com/learn/0.4/reference/hooks#no-hooks-in-loops
    HookInsideLoop(HookInfo, AnyLoopInfo),
    /// https://dioxuslabs.com/learn/0.4/reference/hooks#no-hooks-in-closures
    HookInsideClosure(HookInfo, ClosureInfo),
    HookOutsideComponent(HookInfo),
}

impl Issue {
    pub fn hook_info(&self) -> HookInfo {
        match self {
            Issue::HookInsideConditional(hook_info, _)
            | Issue::HookInsideLoop(hook_info, _)
            | Issue::HookInsideClosure(hook_info, _)
            | Issue::HookOutsideComponent(hook_info) => hook_info.clone(),
        }
    }
}

impl std::fmt::Display for Issue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Issue::HookInsideConditional(hook_info, conditional_info) => {
                write!(
                    f,
                    "hook called conditionally: `{}` (inside `{}`)",
                    hook_info.name,
                    match conditional_info {
                        ConditionalInfo::If(_) => "if",
                        ConditionalInfo::Match(_) => "match",
                    }
                )
            }
            Issue::HookInsideLoop(hook_info, loop_info) => {
                write!(
                    f,
                    "hook called in a loop: `{}` (inside {})",
                    hook_info.name,
                    match loop_info {
                        AnyLoopInfo::For(_) => "`for` loop",
                        AnyLoopInfo::While(_) => "`while` loop",
                        AnyLoopInfo::Loop(_) => "`loop`",
                    }
                )
            }
            Issue::HookInsideClosure(hook_info, _) => {
                write!(f, "hook called in a closure: `{}`", hook_info.name)
            }
            Issue::HookOutsideComponent(hook_info) => {
                write!(
                    f,
                    "hook called outside component or hook: `{}`",
                    hook_info.name
                )
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::check_file;
    use indoc::indoc;
    use pretty_assertions::assert_eq;

    #[test]
    fn test_issue_report_display_conditional_if() {
        owo_colors::set_override(false);
        let issue_report = check_file(
            "src/main.rs".into(),
            indoc! {r#"
                fn App() -> Element {
                    if you_are_happy && you_know_it {
                        let something = use_signal(|| "hands");
                        println!("clap your {something}")
                    }
                }
            "#},
        );

        let expected = indoc! {r#"
            error: hook called conditionally: `use_signal` (inside `if`)
              --> src/main.rs:3:25
              |
            3 |         let something = use_signal(|| "hands");
              |                         ^^^^^^^^^^
              |
              = note: `if you_are_happy && you_know_it { … }` is the conditional
        "#};

        assert_eq!(expected, issue_report.to_string());
    }

    #[test]
    fn test_issue_report_display_conditional_match() {
        owo_colors::set_override(false);
        let issue_report = check_file(
            "src/main.rs".into(),
            indoc! {r#"
                fn App() -> Element {
                    match you_are_happy && you_know_it {
                        true => {
                            let something = use_signal(|| "hands");
                            println!("clap your {something}")
                        }
                        _ => {}
                    }
                }
            "#},
        );

        let expected = indoc! {r#"
            error: hook called conditionally: `use_signal` (inside `match`)
              --> src/main.rs:4:29
              |
            4 |             let something = use_signal(|| "hands");
              |                             ^^^^^^^^^^
              |
              = note: `match you_are_happy && you_know_it { … }` is the conditional
        "#};

        assert_eq!(expected, issue_report.to_string());
    }

    #[test]
    fn test_issue_report_display_for_loop() {
        owo_colors::set_override(false);
        let issue_report = check_file(
            "src/main.rs".into(),
            indoc! {r#"
                fn App() -> Element {
                    for i in 0..10 {
                        let something = use_signal(|| "hands");
                        println!("clap your {something}")
                    }
                }
            "#},
        );

        let expected = indoc! {r#"
            error: hook called in a loop: `use_signal` (inside `for` loop)
              --> src/main.rs:3:25
              |
            3 |         let something = use_signal(|| "hands");
              |                         ^^^^^^^^^^
              |
              = note: `for i in 0..10 { … }` is the loop
        "#};

        assert_eq!(expected, issue_report.to_string());
    }

    #[test]
    fn test_issue_report_display_while_loop() {
        owo_colors::set_override(false);
        let issue_report = check_file(
            "src/main.rs".into(),
            indoc! {r#"
                fn App() -> Element {
                    while check_thing() {
                        let something = use_signal(|| "hands");
                        println!("clap your {something}")
                    }
                }
            "#},
        );

        let expected = indoc! {r#"
            error: hook called in a loop: `use_signal` (inside `while` loop)
              --> src/main.rs:3:25
              |
            3 |         let something = use_signal(|| "hands");
              |                         ^^^^^^^^^^
              |
              = note: `while check_thing() { … }` is the loop
        "#};

        assert_eq!(expected, issue_report.to_string());
    }

    #[test]
    fn test_issue_report_display_loop() {
        owo_colors::set_override(false);
        let issue_report = check_file(
            "src/main.rs".into(),
            indoc! {r#"
                fn App() -> Element {
                    loop {
                        let something = use_signal(|| "hands");
                        println!("clap your {something}")
                    }
                }
            "#},
        );

        let expected = indoc! {r#"
            error: hook called in a loop: `use_signal` (inside `loop`)
              --> src/main.rs:3:25
              |
            3 |         let something = use_signal(|| "hands");
              |                         ^^^^^^^^^^
              |
              = note: `loop { … }` is the loop
        "#};

        assert_eq!(expected, issue_report.to_string());
    }

    #[test]
    fn test_issue_report_display_closure() {
        owo_colors::set_override(false);
        let issue_report = check_file(
            "src/main.rs".into(),
            indoc! {r#"
                fn App() -> Element {
                    let something = || {
                        let something = use_signal(|| "hands");
                        println!("clap your {something}")
                    };
                }
            "#},
        );

        let expected = indoc! {r#"
            error: hook called in a closure: `use_signal`
              --> src/main.rs:3:25
              |
            3 |         let something = use_signal(|| "hands");
              |                         ^^^^^^^^^^
        "#};

        assert_eq!(expected, issue_report.to_string());
    }

    #[test]
    fn test_issue_report_display_multiline_hook() {
        owo_colors::set_override(false);
        let issue_report = check_file(
            "src/main.rs".into(),
            indoc! {r#"
                fn App() -> Element {
                    if you_are_happy && you_know_it {
                        let something = use_signal(|| {
                            "hands"
                        });
                        println!("clap your {something}")
                    }
                }
            "#},
        );

        let expected = indoc! {r#"
            error: hook called conditionally: `use_signal` (inside `if`)
              --> src/main.rs:3:25
              |
            3 |         let something = use_signal(|| {
              |                         ^^^^^^^^^^
            4 |             "hands"
            5 |         });
              |
              = note: `if you_are_happy && you_know_it { … }` is the conditional
        "#};

        assert_eq!(expected, issue_report.to_string());
    }
}
