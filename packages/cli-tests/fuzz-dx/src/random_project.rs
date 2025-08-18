use cargo_toml::Manifest;
use rand::Rng;
use std::io::Write;

pub(crate) fn create_random_project(
    root_path: &std::path::Path,
    dioxus_features: &[String],
) -> Vec<String> {
    std::fs::create_dir_all(&root_path).unwrap();

    let mut manifest = Manifest::from_str(
        r#"[package]
name = "random-dioxus"
version = "0.1.0"
edition = "2024"

[dependencies]
dioxus = { path = "../mock-dioxus" }
# TODO: This shouldn't be required because wasm bindgen is pulled in by mock dioxus
wasm-bindgen = "0.2.100"

[workspace]"#,
    )
    .unwrap();

    // Add a random list of features
    let mut rng = rand::rng();
    for _ in 0..rng.random_range(0..=15) {
        let feature_name = format!("feature_{}", rng.random_range(1..=1000));
        manifest.features.insert(feature_name.clone(), vec![]);
    }
    // Add random connections between features
    let feature_names: Vec<_> = manifest.features.keys().cloned().collect();
    if !feature_names.is_empty() {
        for _ in 0..rng.random_range(0..=15) {
            let feature_a = feature_names[rng.random_range(0..feature_names.len())].clone();
            let feature_b = feature_names[rng.random_range(0..feature_names.len())].clone();
            if feature_a != feature_b {
                manifest
                    .features
                    .get_mut(&feature_a)
                    .unwrap()
                    .push(feature_b);
            }
        }
        // Add random dioxus features
        for _ in 0..rng.random_range(0..=15) {
            let crate_features = feature_names[rng.random_range(0..feature_names.len())].clone();
            let dioxus_feature = dioxus_features
                .get(rng.random_range(0..dioxus_features.len()))
                .unwrap()
                .clone();
            manifest
                .features
                .get_mut(&crate_features)
                .unwrap()
                .push(format!("dioxus/{dioxus_feature}"));
        }
    }

    // Remove any duplicate features
    for features in manifest.features.values_mut() {
        features.sort();
        features.dedup();
    }

    let features = manifest.features.keys().cloned().collect::<Vec<_>>();

    let feature_only_toml_str = toml::to_string(&manifest).unwrap();
    // Create the src folder
    std::fs::create_dir_all(root_path.join("src")).unwrap();
    // Create the Cargo.toml
    std::fs::write(root_path.join("Cargo.toml"), feature_only_toml_str).unwrap();

    // Create the main.rs file
    let file = std::fs::File::create(root_path.join("src/main.rs")).unwrap();
    let mut buf = std::io::BufWriter::new(file);
    writeln!(&mut buf, "fn main() {{").unwrap();
    writeln!(&mut buf, "let mut features = Vec::new();").unwrap();
    for feature in &features {
        writeln!(&mut buf, "    #[cfg(feature = \"{feature}\")]").unwrap();
        writeln!(&mut buf, "    features.push(\"{feature}\".into());").unwrap();
    }
    writeln!(&mut buf, "    dioxus::launch(features);").unwrap();
    writeln!(&mut buf, "}}").unwrap();

    features
}
