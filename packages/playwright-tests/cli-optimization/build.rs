fn main() {
    // use std::path::PathBuf;

    // // If the monaco editor folder doesn't exist, download it
    // let monaco_path = PathBuf::from("monaco-editor");
    // if monaco_path.exists() {
    //     return;
    // }

    // let url = "https://registry.npmjs.org/monaco-editor/-/monaco-editor-0.52.2.tgz";
    // let bytes = reqwest::blocking::get(url).unwrap().bytes().unwrap();
    // let mut archive = tar::Archive::new(flate2::read::GzDecoder::new(bytes.as_ref()));
    // let monaco_path_partial = PathBuf::from("partial-monaco-editor");
    // archive.unpack(&monaco_path_partial).unwrap();
    // std::fs::rename(monaco_path_partial, monaco_path).unwrap();
}
