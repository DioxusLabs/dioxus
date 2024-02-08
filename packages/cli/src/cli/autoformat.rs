use dioxus_autofmt::{IndentOptions, IndentType};
use rayon::prelude::*;
use std::{fs, path::Path, process::exit};

use super::*;

// For reference, the rustfmt main.rs file
// https://github.com/rust-lang/rustfmt/blob/master/src/bin/main.rs

/// Format some rsx
#[derive(Clone, Debug, Parser)]
pub struct Autoformat {
    /// Format rust code before the formatting the rsx macros
    #[clap(long)]
    pub all_code: bool,

    /// Run in 'check' mode. Exits with 0 if input is formatted correctly. Exits
    /// with 1 and prints a diff if formatting is required.
    #[clap(short, long)]
    pub check: bool,

    /// Input rsx (selection)
    #[clap(short, long)]
    pub raw: Option<String>,

    /// Input file
    #[clap(short, long)]
    pub file: Option<String>,

    /// Split attributes in lines or not
    #[clap(short, long, default_value = "false")]
    pub split_line_attributes: bool,
}

impl Autoformat {
    pub fn autoformat(self) -> Result<()> {
        let Autoformat {
            check,
            raw,
            file,
            split_line_attributes,
            all_code: format_rust_code,
            ..
        } = self;

        // Default to formatting the project
        if raw.is_none() && file.is_none() {
            if let Err(e) = autoformat_project(check, split_line_attributes, format_rust_code) {
                eprintln!("error formatting project: {}", e);
                exit(1);
            }
        }

        if let Some(raw) = raw {
            let indent = indentation_for(".", self.split_line_attributes)?;
            if let Some(inner) = dioxus_autofmt::fmt_block(&raw, 0, indent) {
                println!("{}", inner);
            } else {
                // exit process with error
                eprintln!("error formatting codeblock");
                exit(1);
            }
        }

        // Format single file
        if let Some(file) = file {
            refactor_file(file, split_line_attributes, format_rust_code)?;
        }

        Ok(())
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
        eprintln!("failed to open file: {}", file_content.unwrap_err());
        exit(1);
    };

    if format_rust_code {
        s = format_rust(&s)?;
    }

    let edits = dioxus_autofmt::fmt_file(&s, indent);
    let out = dioxus_autofmt::apply_formats(&s, edits);

    if file == "-" {
        print!("{}", out);
    } else if let Err(e) = fs::write(&file, out) {
        eprintln!("failed to write formatted content to file: {e}",);
    } else {
        println!("formatted {}", file);
    }

    Ok(())
}

use std::ffi::OsStr;
fn get_project_files() -> Vec<PathBuf> {
    let mut files = Vec::new();
    for result in ignore::Walk::new("./") {
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
            .map_err(|err| Error::ParseError(format!("Syntax Error:\n{}", err)))?;
        if contents != formatted {
            if_write = true;
            contents = formatted;
        }
    }

    let edits = dioxus_autofmt::fmt_file(&contents, indent);
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
) -> Result<()> {
    let files_to_format = get_project_files();

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
                    eprintln!("error formatting file : {}\n{:#?}", path.display(), err);
                    None
                }
            }
        })
        .collect::<Vec<_>>();

    let files_formatted: usize = counts.into_iter().flatten().sum();

    if files_formatted > 0 && check {
        eprintln!("{} files needed formatting", files_formatted);
        exit(1);
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
        return Err(Error::CargoError("cargo fmt failed".into()));
    }

    let config = String::from_utf8_lossy(&out.stdout);

    let hard_tabs = config
        .lines()
        .find(|line| line.starts_with("hard_tabs "))
        .and_then(|line| line.split_once('='))
        .map(|(_, value)| value.trim() == "true")
        .ok_or_else(|| {
            Error::RuntimeError("Could not find hard_tabs option in rustfmt config".into())
        })?;
    let tab_spaces = config
        .lines()
        .find(|line| line.starts_with("tab_spaces "))
        .and_then(|line| line.split_once('='))
        .map(|(_, value)| value.trim().parse::<usize>())
        .ok_or_else(|| {
            Error::RuntimeError("Could not find tab_spaces option in rustfmt config".into())
        })?
        .map_err(|_| {
            Error::RuntimeError("Could not parse tab_spaces option in rustfmt config".into())
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
    Error::ParseError(format!(
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
    };

    fmt.autoformat().unwrap();
}

/*#[test]
fn spawn_properly() {
    let out = Command::new("dioxus")
        .args([
            "fmt",
            "-f",
            r#"
//

rsx! {

    div {}
}

//
//
//

        "#,
        ])
        .output()
        .expect("failed to execute process");

    dbg!(out);
}*/
