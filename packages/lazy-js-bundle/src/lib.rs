use std::collections::hash_map::DefaultHasher;
use std::path::{Path, PathBuf};
use std::{hash::Hasher, process::Command};

struct Binding {
    input_path: PathBuf,
    output_path: PathBuf,
}

/// A builder for generating TypeScript bindings lazily
#[derive(Default)]
pub struct LazyTypeScriptBindings {
    binding: Vec<Binding>,
    minify_level: MinifyLevel,
    watching: Vec<PathBuf>,
}

impl LazyTypeScriptBindings {
    /// Create a new builder for generating TypeScript bindings that inputs from the given path and outputs javascript to the given path
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a binding to generate
    pub fn with_binding(
        mut self,
        input_path: impl AsRef<Path>,
        output_path: impl AsRef<Path>,
    ) -> Self {
        let input_path = input_path.as_ref();
        let output_path = output_path.as_ref();

        self.binding.push(Binding {
            input_path: input_path.to_path_buf(),
            output_path: output_path.to_path_buf(),
        });

        self
    }

    /// Set the minify level for the bindings
    pub fn with_minify_level(mut self, minify_level: MinifyLevel) -> Self {
        self.minify_level = minify_level;
        self
    }

    /// Watch any .js or .ts files in a directory and re-generate the bindings when they change
    // TODO: we should watch any files that get bundled by bun by reading the source map
    pub fn with_watching(mut self, path: impl AsRef<Path>) -> Self {
        let path = path.as_ref();
        self.watching.push(path.to_path_buf());
        self
    }

    /// Run the bindings
    pub fn run(&self) {
        // If any TS changes, re-run the build script
        let mut watching_paths = Vec::new();
        for path in &self.watching {
            if let Ok(dir) = std::fs::read_dir(path) {
                for entry in dir.flatten() {
                    let path = entry.path();
                    if path
                        .extension()
                        .map(|ext| ext == "ts" || ext == "js")
                        .unwrap_or(false)
                    {
                        watching_paths.push(path);
                    }
                }
            } else {
                watching_paths.push(path.to_path_buf());
            }
        }
        for path in &watching_paths {
            println!("cargo:rerun-if-changed={}", path.display());
        }

        // Compute the hash of the input files
        let hashes = hash_files(watching_paths);

        // Try to find a common prefix for the output files and put the hash in there otherwise, write it to src/binding_hash.txt
        let mut hash_location: Option<PathBuf> = None;
        for path in &self.binding {
            match hash_location {
                Some(current_hash_location) => {
                    let mut common_path = PathBuf::new();
                    for component in path
                        .output_path
                        .components()
                        .zip(current_hash_location.components())
                    {
                        if component.0 != component.1 {
                            break;
                        }
                        common_path.push(component.0);
                    }
                    hash_location =
                        (common_path.components().next().is_some()).then_some(common_path);
                }
                None => {
                    hash_location = Some(path.output_path.clone());
                }
            };
        }
        let hash_location = hash_location.unwrap_or_else(|| PathBuf::from("./src/js"));
        std::fs::create_dir_all(&hash_location).unwrap();
        let hash_location = hash_location.join("hash.txt");

        // If the hash matches the one on disk, we're good and don't need to update bindings
        let fs_hash_string = std::fs::read_to_string(&hash_location);
        let expected = fs_hash_string
            .as_ref()
            .map(|s| s.trim())
            .unwrap_or_default();
        let hashes_string = format!("{hashes:?}");
        if expected == hashes_string {
            return;
        }

        // Otherwise, generate the bindings and write the new hash to disk
        for path in &self.binding {
            gen_bindings(&path.input_path, &path.output_path, self.minify_level);
        }

        std::fs::write(hash_location, hashes_string).unwrap();
    }
}

/// The level of minification to apply to the bindings
#[derive(Copy, Clone, Debug, Default)]
pub enum MinifyLevel {
    /// Don't minify the bindings
    None,
    /// Minify whitespace
    Whitespace,
    /// Minify whitespace and syntax
    #[default]
    Syntax,
    /// Minify whitespace, syntax, and identifiers
    Identifiers,
}

impl MinifyLevel {
    fn as_args(&self) -> &'static [&'static str] {
        match self {
            MinifyLevel::None => &[],
            MinifyLevel::Whitespace => &["--minify-whitespace"],
            MinifyLevel::Syntax => &["--minify-whitespace", "--minify-syntax"],
            MinifyLevel::Identifiers => &[
                "--minify-whitespace",
                "--minify-syntax",
                "--minify-identifiers",
            ],
        }
    }
}

/// Hashes the contents of a directory
fn hash_files(mut files: Vec<PathBuf>) -> Vec<u64> {
    // Different systems will read the files in different orders, so we sort them to make sure the hash is consistent
    files.sort();
    let mut hashes = Vec::new();
    for file in files {
        let mut hash = DefaultHasher::new();
        let Ok(contents) = std::fs::read_to_string(file) else {
            continue;
        };
        // windows + git does a weird thing with line endings, so we need to normalize them
        for line in contents.lines() {
            hash.write(line.as_bytes());
        }
        hashes.push(hash.finish());
    }
    hashes
}

// okay...... so bun might fail if the user doesn't have it installed
// we don't really want to fail if that's the case
// but if you started *editing* the .ts files, you're gonna have a bad time
// so.....
// we need to hash each of the .ts files and add that hash to the JS files
// if the hashes don't match, we need to fail the build
// that way we also don't need
fn gen_bindings(input_path: &Path, output_path: &Path, minify_level: MinifyLevel) {
    // If the file is generated, and the hash is different, we need to generate it
    let status = Command::new("bun")
        .arg("build")
        .arg(input_path)
        .arg("--outfile")
        .arg(output_path)
        .args(minify_level.as_args())
        .status()
        .unwrap();

    if !status.success() {
        panic!(
            "Failed to generate bindings for {:?}. Make sure you have bun installed",
            input_path
        );
    }
}
