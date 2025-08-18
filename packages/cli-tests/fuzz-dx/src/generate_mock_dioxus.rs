use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
};

use cargo_toml::{Dependency, Manifest, Workspace};
use convert_case::Casing;
use toml::Value;

pub(crate) fn generate_mock_dioxus(root_path: &Path) -> Vec<String> {
    let dioxus_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let toml_path = dioxus_path
        .parent()
        .unwrap()
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
    let features_string = features.keys().cloned().collect::<Vec<_>>();

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
            // wasm-bindgen = { version = "0.2.100", optional = true }
            (
                "wasm-bindgen".to_string(),
                // TODO: dx panics if wasm bindgen is optional on web builds, but it shouldn't
                // Dependency::Detailed(Box::new(DependencyDetail {
                //     version: Some("0.2.100".to_string()),
                //     optional: true,
                //     ..Default::default()
                // })),
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
    writeln!(
        &mut buf,
        "pub fn launch(features: impl std::iter::IntoIterator<Item = String>) {{"
    )
    .unwrap();
    writeln!(&mut buf, "    let features_string: Vec<_> = features.into_iter().chain(ENABLED_FEATURES.iter().map(|f| format!(\"dioxus/{{f}}\"))).collect();").unwrap();
    writeln!(&mut buf, "    #[cfg(target_arch = \"wasm32\")]").unwrap();
    writeln!(&mut buf, "    show_features(features_string);").unwrap();
    writeln!(&mut buf, "    #[cfg(not(target_arch = \"wasm32\"))]").unwrap();
    writeln!(
        &mut buf,
        "    std::fs::write(\"features.txt\", features_string.join(\"\\n\")).unwrap();"
    )
    .unwrap();
    writeln!(&mut buf, "    #[cfg(features = \"server\")]").unwrap();
    writeln!(&mut buf, "    launch_server(features_string);").unwrap();

    writeln!(&mut buf, "}}").unwrap();

    let launch_server = r#"#[cfg(features = "server")]
fn launch_server(features_string: Vec<String>) {
    use std::{
        io::{BufReader, prelude::*},
        net::{TcpListener, TcpStream},
    };
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".into());
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port)).unwrap();
    for stream in listener.incoming() {
        let stream = stream.unwrap();
        handle_connection(stream, features_string.clone());
    }
    fn handle_connection(mut stream: TcpStream, features_string: Vec<String>) {
        stream.write_all(b"HTTP/1.1 200 OK\r\n\r\n<html><body><pre>").unwrap();
        stream.write_all(features_string.join("\n").as_bytes()).unwrap();
        stream.write_all(b"</pre></body></html>").unwrap();
    }
}"#;

    writeln!(&mut buf, "{}", launch_server).unwrap();

    features_string
}
