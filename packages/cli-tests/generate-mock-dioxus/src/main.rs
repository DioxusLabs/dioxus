use std::{collections::BTreeMap, io::Write};

use cargo_toml::{Dependency, Manifest, Workspace};
use convert_case::Casing;
use toml::Value;

fn main() {
    let root_path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let root_path = root_path.parent().unwrap();
    let toml_path = root_path
        .parent()
        .unwrap()
        .join("dioxus")
        .join("Cargo.toml");
    let dioxus_crate = Manifest::from_path(toml_path).unwrap();

    let features: BTreeMap<_, Vec<_>> = dioxus_crate
        .features
        .keys()
        .map(|k| (k.clone(), Vec::new()))
        .collect();

    let feature_enum_names = dioxus_crate
        .features
        .keys()
        .map(|f| f.to_case(convert_case::Case::UpperCamel))
        .collect::<Vec<_>>();

    // Create a toml for the mock dioxus crate
    let feature_only_toml = Manifest::<Value> {
        package: dioxus_crate.package,
        features,
        workspace: Some(Workspace {
            members: Default::default(),
            default_members: Default::default(),
            package: Default::default(),
            exclude: Default::default(),
            metadata: Default::default(),
            resolver: Default::default(),
            dependencies: Default::default(),
            lints: Default::default(),
        }),
        dependencies: [
            // wasm-bindgen = "0.2.100"
            (
                "wasm-bindgen".to_string(),
                Dependency::Simple("0.2.100".to_string()),
            ),
        ]
        .into_iter()
        .collect(),
        dev_dependencies: Default::default(),
        build_dependencies: Default::default(),
        target: Default::default(),
        #[allow(deprecated)]
        replace: Default::default(),
        patch: Default::default(),
        lib: Default::default(),
        profile: Default::default(),
        badges: Default::default(),
        bin: Default::default(),
        bench: Default::default(),
        test: Default::default(),
        example: Default::default(),
        lints: Default::default(),
    };

    let feature_only_toml_str = toml::to_string(&feature_only_toml).unwrap();
    // Create a new package folder
    let mock_dioxus_path = root_path.join("mock-dioxus");
    std::fs::create_dir_all(&mock_dioxus_path).unwrap();
    // Create the src folder
    std::fs::create_dir_all(mock_dioxus_path.join("src")).unwrap();
    // Create the Cargo.toml
    std::fs::write(mock_dioxus_path.join("Cargo.toml"), feature_only_toml_str).unwrap();

    // Create the lib.rs file
    let file = std::fs::File::create(mock_dioxus_path.join("src/lib.rs")).unwrap();
    let mut buf = std::io::BufWriter::new(file);
    writeln!(&mut buf, "pub enum Feature {{").unwrap();
    for feature in &feature_enum_names {
        writeln!(&mut buf, "    {},", feature).unwrap();
    }
    writeln!(&mut buf, "}}").unwrap();

    writeln!(&mut buf, "impl std::fmt::Display for Feature {{").unwrap();
    writeln!(
        &mut buf,
        "    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {{"
    )
    .unwrap();
    writeln!(&mut buf, "        match self {{").unwrap();
    for (feature, feature_enum_name) in dioxus_crate.features.keys().zip(&feature_enum_names) {
        writeln!(
            &mut buf,
            "            Feature::{feature_enum_name} => std::fmt::Display::fmt({feature:?}, f),",
        )
        .unwrap();
    }
    writeln!(&mut buf, "        }}").unwrap();
    writeln!(&mut buf, "    }}").unwrap();
    writeln!(&mut buf, "}}").unwrap();

    writeln!(&mut buf, "pub static ENABLED_FEATURES: &[Feature] = &[").unwrap();
    for (feature, feature_enum_name) in dioxus_crate.features.keys().zip(&feature_enum_names) {
        writeln!(&mut buf, "    #[cfg(feature = \"{feature}\")]").unwrap();
        writeln!(&mut buf, "    Feature::{feature_enum_name},").unwrap();
    }
    writeln!(&mut buf, "];").unwrap();

    writeln!(&mut buf, "use wasm_bindgen::prelude::*;").unwrap();
    writeln!(
        &mut buf,
        "#[wasm_bindgen(inline_js = \"export function show_features(features) {{
            let pre = document.createElement('pre');
            pre.setAttribute('id', 'features');
            pre.innerText = features.join('\\\\n');
            document.body.appendChild(pre);
        }}\")]"
    )
    .unwrap();
    writeln!(&mut buf, "extern \"C\" {{").unwrap();
    writeln!(&mut buf, "    fn show_features(features: Vec<String>);").unwrap();
    writeln!(&mut buf, "}}").unwrap();

    writeln!(&mut buf, "\n").unwrap();
    writeln!(&mut buf, "pub fn launch() {{").unwrap();
    writeln!(&mut buf, "    let features_string: Vec<_> = ENABLED_FEATURES.iter().map(|f| f.to_string()).collect();").unwrap();
    writeln!(&mut buf, "    #[cfg(target_arch = \"wasm32\")]").unwrap();
    writeln!(&mut buf, "    show_features(features_string);").unwrap();
    writeln!(&mut buf, "    #[cfg(not(target_arch = \"wasm32\"))]").unwrap();
    writeln!(
        &mut buf,
        "    std::fs::write(\"features.txt\", features_string.join(\"\\n\")).unwrap();"
    )
    .unwrap();
    writeln!(&mut buf, "}}").unwrap();
}
