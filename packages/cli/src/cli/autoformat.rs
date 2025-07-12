use super::{check::collect_rs_files, *};
use crate::Workspace;
use anyhow::{bail, Context};
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

    /// Input file at path (set to "-" to read file from stdin, and output formatted file to stdout)
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
    pub(crate) async fn autoformat(self) -> Result<StructuredOutput> {
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
            let formatted =
                dioxus_autofmt::fmt_block(&raw, 0, indent).context("error formatting codeblock")?;
            println!("{}", formatted);
        } else {
            // Default to formatting the project.
            let crate_dir = if let Some(package) = self.package {
                let workspace = Workspace::current().await?;
                let dx_crate = workspace
                    .find_main_package(Some(package))
                    .context("Failed to find package")?;
                workspace.krates[dx_crate]
                    .manifest_path
                    .parent()
                    .unwrap()
                    .to_path_buf()
                    .into()
            } else {
                Cow::Borrowed(Path::new("."))
            };

            autoformat_project(check, split_line_attributes, format_rust_code, crate_dir)
                .context("error autoformatting project")?;
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
    let mut s = file_content.context("failed to open file")?;

    if format_rust_code {
        s = format_rust(&s)?;
    }

    let parsed = syn::parse_file(&s).context("failed to parse file")?;
    let edits =
        dioxus_autofmt::try_fmt_file(&s, &parsed, indent).context("failed to format file")?;

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

fn format_file(
    path: impl AsRef<Path>,
    indent: IndentOptions,
    format_rust_code: bool,
) -> Result<usize> {
    let mut contents = fs::read_to_string(&path)?;
    let mut if_write = false;
    if format_rust_code {
        let formatted = format_rust(&contents).context("Syntax Error")?;
        if contents != formatted {
            if_write = true;
            contents = formatted;
        }
    }

    let parsed = syn::parse_file(&contents).context("Failed to parse file")?;
    let edits = dioxus_autofmt::try_fmt_file(&contents, &parsed, indent)
        .context("Failed to format file")?;
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
    let mut files_to_format = vec![];
    collect_rs_files(dir.as_ref(), &mut files_to_format);

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
        bail!("{} files needed formatting", files_formatted);
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
        bail!("cargo fmt failed with status: {out:?}");
    }

    let config = String::from_utf8_lossy(&out.stdout);

    let hard_tabs = config
        .lines()
        .find(|line| line.starts_with("hard_tabs "))
        .and_then(|line| line.split_once('='))
        .map(|(_, value)| value.trim() == "true")
        .context("Could not find hard_tabs option in rustfmt config")?;
    let tab_spaces = config
        .lines()
        .find(|line| line.starts_with("tab_spaces "))
        .and_then(|line| line.split_once('='))
        .map(|(_, value)| value.trim().parse::<usize>())
        .context("Could not find tab_spaces option in rustfmt config")?
        .context("Could not parse tab_spaces option in rustfmt config")?;

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
    let syntax_tree = syn::parse_file(input)
        .map_err(format_syn_error)
        .context("Failed to parse Rust syntax")?;
    let output = prettyplease::unparse(&syntax_tree);
    Ok(output)
}

fn format_syn_error(err: syn::Error) -> Error {
    let start = err.span().start();
    let line = start.line;
    let column = start.column;
    anyhow::anyhow!("Syntax Error in line {} column {}:\n{}", line, column, err)
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

    fmt.autoformat().await.unwrap();
}
