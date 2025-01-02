use crate::Result;
use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::process::Command;

#[cfg(not(feature = "no-downloads"))]
mod managed;
#[cfg(feature = "no-downloads")]
mod path;

#[cfg(not(feature = "no-downloads"))]
type Binary = managed::ManagedBinary;
#[cfg(feature = "no-downloads")]
type Binary = path::PathBinary;

pub(crate) trait WasmBindgenBinary {
    fn new(version: &str) -> Self;
    async fn verify_install(&self) -> anyhow::Result<()>;
    async fn get_binary_path(&self) -> anyhow::Result<PathBuf>;
}

pub(crate) struct WasmBindgen {
    version: String,
    input_path: PathBuf,
    out_dir: PathBuf,
    out_name: String,
    target: String,
    debug: bool,
    keep_debug: bool,
    demangle: bool,
    remove_name_section: bool,
    remove_producers_section: bool,
}

impl WasmBindgen {
    pub async fn run(&self) -> Result<()> {
        let binary = Binary::new(&self.version).get_binary_path().await?;
        let mut args = Vec::new();

        // Target
        args.push("--target");
        args.push(&self.target);

        // Options
        if self.debug {
            args.push("--debug");
        }

        if !self.demangle {
            args.push("--no-demangle");
        }

        if self.keep_debug {
            args.push("--keep-debug");
        }

        if self.remove_name_section {
            args.push("--remove-name-section");
        }

        if self.remove_producers_section {
            args.push("--remove-producers-section");
        }

        // Out name
        args.push("--out-name");
        args.push(&self.out_name);

        // Out dir
        let out_dir = self
            .out_dir
            .to_str()
            .expect("input_path should be valid utf8");

        args.push("--out-dir");
        args.push(out_dir);

        // Input path
        let input_path = self
            .input_path
            .to_str()
            .expect("input_path should be valid utf8");
        args.push(input_path);

        // Run bindgen
        Command::new(binary)
            .args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        Ok(())
    }

    pub async fn verify_install(version: &str) -> anyhow::Result<()> {
        Binary::new(version).verify_install().await
    }
}

/// A builder for WasmBindgen options.
pub(crate) struct WasmBindgenBuilder {
    version: String,
    input_path: PathBuf,
    out_dir: PathBuf,
    out_name: String,
    target: String,
    debug: bool,
    keep_debug: bool,
    demangle: bool,
    remove_name_section: bool,
    remove_producers_section: bool,
}

impl WasmBindgenBuilder {
    pub fn new(version: String) -> Self {
        Self {
            version,
            input_path: PathBuf::new(),
            out_dir: PathBuf::new(),
            out_name: String::new(),
            target: String::new(),
            debug: true,
            keep_debug: true,
            demangle: true,
            remove_name_section: false,
            remove_producers_section: false,
        }
    }

    pub fn build(self) -> WasmBindgen {
        WasmBindgen {
            version: self.version,
            input_path: self.input_path,
            out_dir: self.out_dir,
            out_name: self.out_name,
            target: self.target,
            debug: self.debug,
            keep_debug: self.keep_debug,
            demangle: self.demangle,
            remove_name_section: self.remove_name_section,
            remove_producers_section: self.remove_producers_section,
        }
    }

    pub fn input_path(self, input_path: &Path) -> Self {
        Self {
            input_path: input_path.to_path_buf(),
            ..self
        }
    }

    pub fn out_dir(self, out_dir: &Path) -> Self {
        Self {
            out_dir: out_dir.to_path_buf(),
            ..self
        }
    }

    pub fn out_name(self, out_name: &str) -> Self {
        Self {
            out_name: out_name.to_string(),
            ..self
        }
    }

    pub fn target(self, target: &str) -> Self {
        Self {
            target: target.to_string(),
            ..self
        }
    }

    pub fn debug(self, debug: bool) -> Self {
        Self { debug, ..self }
    }

    pub fn keep_debug(self, keep_debug: bool) -> Self {
        Self { keep_debug, ..self }
    }

    pub fn demangle(self, demangle: bool) -> Self {
        Self { demangle, ..self }
    }

    pub fn remove_name_section(self, remove_name_section: bool) -> Self {
        Self {
            remove_name_section,
            ..self
        }
    }

    pub fn remove_producers_section(self, remove_producers_section: bool) -> Self {
        Self {
            remove_producers_section,
            ..self
        }
    }
}
