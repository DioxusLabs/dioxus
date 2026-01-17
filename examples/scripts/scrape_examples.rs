use std::path::PathBuf;

struct Example {
    name: String,
    path: PathBuf,
}

fn main() {
    let dir = PathBuf::from("/Users/jonathankelley/Development/dioxus/examples");
    let out_file = PathBuf::from("/Users/jonathankelley/Development/dioxus/target/decl.toml");

    let mut out_items = vec![];

    // Iterate through the sub-directories of the examples directory
    for dir in dir.read_dir().unwrap().flatten() {
        // For each sub-directory, walk it too, collecting .rs files
        let Ok(dir) = dir.path().read_dir() else {
            continue;
        };

        for dir in dir.flatten() {
            let path = dir.path();
            if path.extension().and_then(|s| s.to_str()) == Some("rs") {
                let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap();
                let workspace_path = path
                    .strip_prefix("/Users/jonathankelley/Development/dioxus")
                    .unwrap();
                out_items.push(Example {
                    name: file_stem.to_string(),
                    path: workspace_path.to_path_buf(),
                });
            }
        }
    }

    let mut out_toml = String::new();
    out_items.sort_by(|a, b| a.path.cmp(&b.path));

    for item in out_items {
        out_toml.push_str(&format!(
            "[[example]]\nname = \"{}\"\npath = \"{}\"\ndoc-scrape-examples = true\n\n",
            item.name,
            item.path.display()
        ));
    }

    std::fs::write(out_file, out_toml).unwrap();
}
