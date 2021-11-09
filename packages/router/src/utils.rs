use wasm_bindgen::JsCast;
use web_sys::window;

pub fn fetch_base_url() -> Option<String> {
    match window()
        .unwrap()
        .document()
        .unwrap()
        .query_selector("base[href]")
    {
        Ok(Some(base)) => {
            let base = JsCast::unchecked_into::<web_sys::HtmlBaseElement>(base).href();

            let url = web_sys::Url::new(&base).unwrap();
            let base = url.pathname();

            let base = if base != "/" {
                strip_slash_suffix(&base)
            } else {
                return None;
            };

            Some(base.to_string())
        }
        _ => None,
    }
}

pub(crate) fn strip_slash_suffix(path: &str) -> &str {
    path.strip_suffix('/').unwrap_or(path)
}
