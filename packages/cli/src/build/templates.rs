#[derive(serde::Serialize)]
pub struct InfoPlistData {
    pub display_name: String,
    pub bundle_name: String,
    pub bundle_identifier: String,
    pub executable_name: String,
}
