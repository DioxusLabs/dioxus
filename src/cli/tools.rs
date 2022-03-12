use std::path::PathBuf;

pub enum Tool {
    WasmOpt,
}

pub fn app_path() {
    let data_local = dirs::data_local_dir().unwrap();
    
}

impl Tool {
    pub fn name(&self) -> &str {
        match self {
            Self::WasmOpt => "wasm-opt",
        }
    }

    pub fn bin_path(&self) -> &str {
        if cfg!(target_os = "windows") {
            match self {
                Self::WasmOpt => "bin/wasm-opt.exe",
            }
        } else {
            match self {
                Self::WasmOpt => "bin/wasm-opt",
            }
        }
    }

    pub fn target_platform(&self) -> &str {
        match self {
            Self::WasmOpt => {
                if cfg!(target_os = "windows") {
                    "windows"
                } else if cfg!(target_os = "macos") {
                    "macos"
                } else if cfg!(target_os = "linux") {
                    "linux"
                } else {
                    panic!("unsupported platformm");
                }
            }
        }
    }

    pub fn download_url(&self) -> String {
        match self {
            Self::WasmOpt => {
                format!(
                    "https://github.com/WebAssembly/binaryen/releases/download/version_105/binaryen-version_105-x86_64-{target}.tar.gz",
                    target = self.target_platform()
                )
            }
        }
    }

    pub async fn download_package(&self) {
        
        let download_dir = dirs::download_dir().unwrap();
        let download_url = self.download_url();

        let resp = reqwest::get(download_url).await.unwrap();

        resp.

    }

}
