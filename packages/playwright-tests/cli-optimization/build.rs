use std::path::PathBuf;

fn main() {
    // If the monaco editor folder doesn't exist, download it
    let monaco_path = PathBuf::from("monaco-editor-0.52.2");
    if monaco_path.exists() {
        return;
    }

    let url = "https://registry.npmjs.org/monaco-editor/-/monaco-editor-0.52.2.tgz";
    let bytes = reqwest::blocking::get(url).unwrap().bytes().unwrap();
    let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes.as_ref()));
    archive.unpack(&monaco_path).unwrap();
}
