use super::*;
use crate::DioxusCrate;
use anyhow::Context;
use dioxus_autofmt::{IndentOptions, IndentType};
use rayon::prelude::*;
use std::{borrow::Cow, fs, path::Path};

// For reference, the rustfmt main.rs file
// https://github.com/rust-lang/rustfmt/blob/master/src/bin/main.rs

/// Format some rsx
#[derive(Clone, Debug, Parser)]
pub(crate) struct Autoformat {
    /// Format rust code before the formatting the rsx macros
    #[clap(long)]
    pub(crate) all_code: bool,

    /// Run in 'check' mode. Exits with 0 if input is formatted correctly. Exits
    /// with 1 and prints a diff if formatting is required.
    #[clap(short, long)]
    pub(crate) check: bool,

    /// Input rsx (selection)
    #[clap(short, long)]
    pub(crate) raw: Option<String>,

    /// Input file
    #[clap(short, long)]
    pub(crate) file: Option<String>,

    /// Split attributes in lines or not
    #[clap(short, long, default_value = "false")]
    pub(crate) split_line_attributes: bool,

    /// The package to build
    #[clap(short, long)]
    pub(crate) package: Option<String>,
}

impl Autoformat {
    pub(crate) fn autoformat(self) -> Result<StructuredOutput> {
        let Autoformat {
            check,
            raw,
            file,
            split_line_attributes,
            all_code: format_rust_code,
            ..
        } = self;

        if let Some(file) = file {
            // Format a single file
            refactor_file(file, split_line_attributes, format_rust_code)?;
        } else if let Some(raw) = raw {
            // Format raw text.
            let indent = indentation_for(".", self.split_line_attributes)?;
            if let Some(inner) = dioxus_autofmt::fmt_block(&raw, 0, indent) {
                println!("{}", inner);
            } else {
                return Err("error formatting codeblock".into());
            }
        } else {
            // Default to formatting the project.
            let crate_dir = if let Some(package) = self.package {
                // TODO (matt): Do we need to use the entire `DioxusCrate` here?
                let target_args = TargetArgs {
                    package: Some(package),
                    ..Default::default()
                };
                let dx_crate =
                    DioxusCrate::new(&target_args).context("failed to parse crate graph")?;

                Cow::Owned(dx_crate.crate_dir())
            } else {
                Cow::Borrowed(Path::new("."))
            };

            if let Err(e) =
                autoformat_project(check, split_line_attributes, format_rust_code, crate_dir)
            {
                return Err(format!("error formatting project: {}", e).into());
            }
        }

        Ok(StructuredOutput::Success)
    }
}

fn refactor_file(
    file: String,
    split_line_attributes: bool,
    format_rust_code: bool,
) -> Result<(), Error> {
    let indent = indentation_for(".", split_line_attributes)?;
    let file_content = if file == "-" {
        let mut contents = String::new();
        std::io::stdin().read_to_string(&mut contents)?;
        Ok(contents)
    } else {
        fs::read_to_string(&file)
    };
    let Ok(mut s) = file_content else {
        return Err(format!("failed to open file: {}", file_content.unwrap_err()).into());
    };

    if format_rust_code {
        s = format_rust(&s)?;
    }

    let Ok(Ok(edits)) =
        syn::parse_file(&s).map(|file| dioxus_autofmt::try_fmt_file(&s, &file, indent))
    else {
        return Err(format!("failed to format file: {}", s).into());
    };

    let out = dioxus_autofmt::apply_formats(&s, edits);

    if file == "-" {
        print!("{}", out);
    } else if let Err(e) = fs::write(&file, out) {
        tracing::error!("failed to write formatted content to file: {e}",);
    } else {
        println!("formatted {}", file);
    }

    Ok(())
}

use std::ffi::OsStr;
fn get_project_files(dir: impl AsRef<Path>) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for result in ignore::Walk::new(dir) {
        let path = result.unwrap().into_path();
        if let Some(ext) = path.extension() {
            if ext == OsStr::new("rs") {
                files.push(path);
            }
        }
    }
    files
}

fn format_file(
    path: impl AsRef<Path>,
    indent: IndentOptions,
    format_rust_code: bool,
) -> Result<usize> {
    let mut contents = fs::read_to_string(&path)?;
    let mut if_write = false;
    if format_rust_code {
        let formatted = format_rust(&contents)
            .map_err(|err| Error::Parse(format!("Syntax Error:\n{}", err)))?;
        if contents != formatted {
            if_write = true;
            contents = formatted;
        }
    }

    let parsed = syn::parse_file(&contents)
        .map_err(|err| Error::Parse(format!("Failed to parse file: {}", err)))?;
    let edits = dioxus_autofmt::try_fmt_file(&contents, &parsed, indent)
        .map_err(|err| Error::Parse(format!("Failed to format file: {}", err)))?;
    let len = edits.len();

    if !edits.is_empty() {
        if_write = true;
    }

    if if_write {
        let out = dioxus_autofmt::apply_formats(&contents, edits);
        fs::write(path, out)?;
    }

    Ok(len)
}

/// Read every .rs file accessible when considering the .gitignore and try to format it
///
/// Runs using rayon for multithreading, so it should be really really fast
///
/// Doesn't do mod-descending, so it will still try to format unreachable files. TODO.
fn autoformat_project(
    check: bool,
    split_line_attributes: bool,
    format_rust_code: bool,
    dir: impl AsRef<Path>,
) -> Result<()> {
    let files_to_format = get_project_files(dir);

    if files_to_format.is_empty() {
        return Ok(());
    }

    if files_to_format.is_empty() {
        return Ok(());
    }

    let indent = indentation_for(&files_to_format[0], split_line_attributes)?;

    let counts = files_to_format
        .into_par_iter()
        .map(|path| {
            let res = format_file(&path, indent.clone(), format_rust_code);
            match res {
                Ok(cnt) => Some(cnt),
                Err(err) => {
                    tracing::error!("error formatting file : {}\n{:#?}", path.display(), err);
                    None
                }
            }
        })
        .collect::<Vec<_>>();

    let files_formatted: usize = counts.into_iter().flatten().sum();

    if files_formatted > 0 && check {
        return Err(format!("{} files needed formatting", files_formatted).into());
    }

    Ok(())
}

fn indentation_for(
    file_or_dir: impl AsRef<Path>,
    split_line_attributes: bool,
) -> Result<IndentOptions> {
    let out = std::process::Command::new("cargo")
        .args(["fmt", "--", "--print-config", "current"])
        .arg(file_or_dir.as_ref())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::inherit())
        .output()?;

    if !out.status.success() {
        return Err(Error::Runtime(format!(
            "cargo fmt failed with status: {out:?}"
        )));
    }

    let config = String::from_utf8_lossy(&out.stdout);

    let hard_tabs = config
        .lines()
        .find(|line| line.starts_with("hard_tabs "))
        .and_then(|line| line.split_once('='))
        .map(|(_, value)| value.trim() == "true")
        .ok_or_else(|| {
            Error::Runtime("Could not find hard_tabs option in rustfmt config".into())
        })?;
    let tab_spaces = config
        .lines()
        .find(|line| line.starts_with("tab_spaces "))
        .and_then(|line| line.split_once('='))
        .map(|(_, value)| value.trim().parse::<usize>())
        .ok_or_else(|| Error::Runtime("Could not find tab_spaces option in rustfmt config".into()))?
        .map_err(|_| {
            Error::Runtime("Could not parse tab_spaces option in rustfmt config".into())
        })?;

    Ok(IndentOptions::new(
        if hard_tabs {
            IndentType::Tabs
        } else {
            IndentType::Spaces
        },
        tab_spaces,
        split_line_attributes,
    ))
}

/// Format rust code using prettyplease
fn format_rust(input: &str) -> Result<String> {
    let syntax_tree = syn::parse_file(input).map_err(format_syn_error)?;
    let output = prettyplease::unparse(&syntax_tree);
    Ok(output)
}

fn format_syn_error(err: syn::Error) -> Error {
    let start = err.span().start();
    let line = start.line;
    let column = start.column;
    Error::Parse(format!(
        "Syntax Error in line {} column {}:\n{}",
        line, column, err
    ))
}

#[tokio::test]
async fn test_auto_fmt() {
    let test_rsx = r#"
                    //



                        div {}


                    //
                    //
                    //

                    "#
    .to_string();

    let fmt = Autoformat {
        all_code: false,
        check: false,
        raw: Some(test_rsx),
        file: None,
        split_line_attributes: false,
        package: None,
    };

    fmt.autoformat().unwrap();
}
