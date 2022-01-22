use crate::error::Result;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct CrateConfig {
    pub out_dir: PathBuf,
    pub crate_dir: PathBuf,
    pub workspace_dir: PathBuf,
    pub target_dir: PathBuf,
    pub static_dir: PathBuf,
    pub manifest: cargo_toml::Manifest<cargo_toml::Value>,
    pub executable: ExecutableType,
    pub release: bool,
}

#[derive(Debug, Clone)]
pub enum ExecutableType {
    Binary(String),
    Lib(String),
    Example(String),
}

impl CrateConfig {
    pub fn new() -> Result<Self> {
        let crate_dir = crate::cargo::crate_root()?;
        let workspace_dir = crate::cargo::workspace_root()?;
        let target_dir = workspace_dir.join("target");
        let out_dir = crate_dir.join("public");
        let cargo_def = &crate_dir.join("Cargo.toml");
        let static_dir = crate_dir.join("static");

        let manifest = cargo_toml::Manifest::from_path(&cargo_def).unwrap();

        // We just assume they're using a 'main.rs'
        // Anyway, we've already parsed the manifest, so it should be easy to change the type
        let output_filename = (&manifest)
            .lib
            .as_ref()
            .and_then(|lib| lib.name.clone())
            .or_else(|| {
                (&manifest)
                    .package
                    .as_ref()
                    .and_then(|pkg| Some(pkg.name.replace("-", "_")))
                    .clone()
            })
            .expect("No lib found from cargo metadata");
        let executable = ExecutableType::Binary(output_filename);

        let release = false;

        Ok(Self {
            out_dir,
            crate_dir,
            workspace_dir,
            target_dir,
            static_dir,
            manifest,
            executable,
            release,
        })
    }

    pub fn as_example(&mut self, example_name: String) -> &mut Self {
        self.executable = ExecutableType::Example(example_name);
        self
    }

    pub fn with_release(&mut self, release: bool) -> &mut Self {
        self.release = release;
        self
    }

    // pub fn with_build_options(&mut self, options: &BuildOptions) {
    //     if let Some(name) = &options.example {
    //         self.as_example(name.clone());
    //     }
    //     self.release = options.release;
    //     self.out_dir = options.outdir.clone().into();
    // }

    // pub fn with_develop_options(&mut self, options: &DevelopOptions) {
    //     if let Some(name) = &options.example {
    //         self.as_example(name.clone());
    //     }
    //     self.release = options.release;
    //     self.out_dir = tempfile::Builder::new().tempdir().expect("").into_path();
    // }
}
