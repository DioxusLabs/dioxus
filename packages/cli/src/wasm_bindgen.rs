use std::{
    path::{Path, PathBuf},
    process::Stdio,
};
use tokio::{fs, process::Command};

pub(crate) struct WasmBindgen {
    input_path: PathBuf,
    out_dir: PathBuf,
    out_name: String,
    web: bool,
    debug: bool,
    keep_debug: bool,
    demangle: bool,
    remove_name_section: bool,
    remove_producers_section: bool,
}

impl WasmBindgen {
    pub fn verify_install(version: String) -> anyhow::Result<bool> {
        todo!()
    }

    /// Get the github install url.
    fn git_install_url(version: String) -> Option<String> {
        let platform = if cfg!(all(target_os = "windows", target_arch = "x86_64")) {
            "x86_64-pc-windows-msvc.tar.gz"
        } else if cfg!(all(target_os = "linux", target_arch = "x86_64")) {
            "x86_64-unknown-linux-musl.tar.gz"
        } else if cfg!(all(target_os = "linux", target_arch = "aarch64")) {
            "aarch64-unknown-linux-gnu.tar.gz"
        } else if cfg!(all(target_os = "macos", target_arch = "x86_64")) {
            "x86_64-apple-darwin.tar.gz"
        } else if cfg!(all(target_os = "macos", target_arch = "aarch64")) {
            "aarch64-apple-darwin.tar.gz"
        } else {
            return None;
        };

        Some(format!("https://github.com/rustwasm/wasm-bindgen/releases/download/{version}/wasm-bindgen-{version}-{platform}.tar.gz"))
    }

    /// Try installing wasm-bindgen through cargo-binstall.
    async fn install_binstall(version: String) -> anyhow::Result<PathBuf> {
        let package = format!("wasm-bindgen-cli@{version}");
        let tmp_dir = std::env::temp_dir();
        let install_dir = Self::install_dir();

        // Run install command
        Command::new("cargo")
            .args([
                "binstall",
                &package,
                "--no-confirm",
                "--no-track",
                "--install-path",
                tmp_dir.to_str().expect("this should be utf8-compatable"),
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await?;

        // Get the final binary location.
        let mut installed_name = format!("wasm-bindgen-{version}");
        if cfg!(windows) {
            installed_name = format!("{installed_name}.exe");
        }
        let final_binary = install_dir.join(installed_name);

        // Move the install wasm-bindgen binary from tmp directory to it's new location.
        let mut tmp_install_name = "wasm-bindgen";
        if cfg!(windows) {
            tmp_install_name = "wasm-bindgen.exe";
        }
        fs::copy(tmp_dir.join(tmp_install_name), &final_binary).await?;

        Ok(final_binary)
    }

    /// Install wasm-bindgen from source using cargo install.
    async fn install_cargo() -> anyhow::Result<()> {
        // note use --root tmp_dir
        todo!()
    }

    /// Get the installation directory for the wasm-bindgen executable.
    fn install_dir() -> PathBuf {
        todo!()
    }
}

pub(crate) struct WasmBindgenBuilder {
    input_path: PathBuf,
    out_dir: PathBuf,
    out_name: String,
    web: bool,
    debug: bool,
    keep_debug: bool,
    demangle: bool,
    remove_name_section: bool,
    remove_producers_section: bool,
}

impl WasmBindgenBuilder {
    pub fn new() -> Self {
        Self {
            input_path: PathBuf::new(),
            out_dir: PathBuf::new(),
            out_name: "bindgen".to_string(),
            web: true,
            debug: true,
            keep_debug: true,
            demangle: true,
            remove_name_section: false,
            remove_producers_section: false,
        }
    }

    pub fn build(self) -> WasmBindgen {
        WasmBindgen {
            input_path: self.input_path,
            out_dir: self.out_dir,
            out_name: self.out_name,
            web: self.web,
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

    pub fn out_name(self, out_name: String) -> Self {
        Self { out_name, ..self }
    }

    pub fn web(self, web: bool) -> Self {
        Self { web, ..self }
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
