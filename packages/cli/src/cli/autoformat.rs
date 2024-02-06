use dioxus_autofmt::{IndentOptions, IndentType};
use rayon::prelude::*;
use std::{fs, path::Path, process::exit};

use super::*;

// For reference, the rustfmt main.rs file
// https://github.com/rust-lang/rustfmt/blob/master/src/bin/main.rs

/// Format some rsx
#[derive(Clone, Debug, Parser)]
pub struct Autoformat {
    /// Run rustfmt before the dioxus formatter`
    #[clap(long)]
    pub rustfmt: bool,

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
    // Todo: autoformat the entire crate
    pub async fn autoformat(self) -> Result<()> {
        let Autoformat {
            check,
            raw,
            file,
            split_line_attributes,
            rustfmt: do_rustfmt,
            ..
        } = self;

        // Default to formatting the project
        if raw.is_none() && file.is_none() {
            println!("format project !");
            if let Err(e) = autoformat_project(check, split_line_attributes, do_rustfmt).await {
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
            refactor_file(file, split_line_attributes, do_rustfmt)?;
        }

        Ok(())
    }
}

fn refactor_file(file: String, split_line_attributes: bool, do_rustfmt: bool) -> Result<(), Error> {
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

    if do_rustfmt {
        s = dioxus_autofmt::rustfmt(&s).ok_or_else(|| Error::ParseError("Syntax Error".into()))?;
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

fn format_file(path: impl AsRef<Path>, indent: IndentOptions, do_rustfmt: bool) -> Result<usize> {
    let mut contents = fs::read_to_string(&path)?;
    let mut if_write = false;
    if do_rustfmt {
        let formatted = dioxus_autofmt::rustfmt(&contents)
            .ok_or_else(|| Error::ParseError("Syntax Error".into()))?;
        if contents != formatted {
            if_write = true;
            contents = formatted;
        }
    }

    println!("at {} : {:#?}", path.as_ref().display(), &contents);

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
/// Runs using Tokio for multithreading, so it should be really really fast
///
/// Doesnhttps://www.rustwiki.org.cn/zh-CN/std/'t do mod-descending, so it will still try to format unreachable files. TODO.
async fn autoformat_project(
    check: bool,
    split_line_attributes: bool,
    do_rustfmt: bool,
) -> Result<()> {
    let files_to_format = get_project_files();

    if files_to_format.is_empty() {
        return Ok(());
    }

    let indent = indentation_for(&files_to_format[0], split_line_attributes)?;

    let counts = files_to_format
        .into_par_iter()
        .map(|path| {
            let res = format_file(&path, indent.clone(), do_rustfmt);
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
        rustfmt: false,
        check: false,
        raw: Some(test_rsx),
        file: None,
        split_line_attributes: false,
    };

    fmt.autoformat().await.unwrap();
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
