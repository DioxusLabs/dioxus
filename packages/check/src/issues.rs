use owo_colors::{
    colors::{css::LightBlue, BrightRed},
    OwoColorize,
};
use std::{
    fmt::Display,
    path::{Path, PathBuf},
};

use crate::metadata::{AnyLoopInfo, ClosureInfo, ConditionalInfo, HookInfo};

/// The result of checking a Dioxus file for issues.
pub struct IssueReport {
    pub path: PathBuf,
    pub file_content: String,
    pub issues: Vec<Issue>,
}

impl IssueReport {
    pub fn new<S: ToString>(path: PathBuf, file_content: S, issues: Vec<Issue>) -> Self {
        Self {
            path,
            file_content: file_content.to_string(),
            issues,
        }
    }
}

impl Display for IssueReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let relative_file = Path::new(&self.path)
            .strip_prefix(std::env::current_dir().unwrap())
            .unwrap_or(Path::new(&self.path))
            .display();

        for (i, issue) in self.issues.iter().enumerate() {
            let hook_info = issue.hook_info();
            let hook_span = hook_info.span;
            let hook_name_span = hook_info.name_span;
            let error_line = format!("{}: {}", "error".fg::<BrightRed>(), issue);
            writeln!(f, "{}", error_line.bold())?;
            writeln!(
                f,
                "  {} {}:{}:{}",
                "-->".fg::<LightBlue>(),
                relative_file,
                hook_span.start.line,
                hook_span.start.column + 1
            )?;
            let max_line_num_len = hook_span.end.line.to_string().len();
            writeln!(f, "{:>max_line_num_len$} {}", "", "|".fg::<LightBlue>())?;
            for (i, line) in self.file_content.lines().enumerate() {
                let line_num = i + 1;
                if line_num >= hook_span.start.line && line_num <= hook_span.end.line {
                    writeln!(
                        f,
                        "{:>max_line_num_len$} {} {}",
                        line_num.fg::<LightBlue>(),
                        "|".fg::<LightBlue>(),
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
                            "|".fg::<LightBlue>(),
                            caret.fg::<BrightRed>(),
                        )?;
                    }
                }
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
    /// https://dioxuslabs.com/docs/0.3/guide/en/interactivity/hooks.html#no-hooks-in-conditionals
    HookInsideConditional(HookInfo, ConditionalInfo),
    /// https://dioxuslabs.com/docs/0.3/guide/en/interactivity/hooks.html#no-hooks-in-loops
    HookInsideLoop(HookInfo, AnyLoopInfo),
    /// https://dioxuslabs.com/docs/0.3/guide/en/interactivity/hooks.html#no-hooks-in-closures
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
                write!(f, "hook called inside closure: `{}`", hook_info.name)
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
