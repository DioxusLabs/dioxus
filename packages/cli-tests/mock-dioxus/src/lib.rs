pub enum Feature {
    Asset,
    CliConfig,
    Default,
    Desktop,
    Devtools,
    Document,
    FileEngine,
    Fullstack,
    Hooks,
    Html,
    Launch,
    Lib,
    Liveview,
    Logger,
    Macro,
    Minimal,
    Mobile,
    Mounted,
    Native,
    Router,
    Server,
    Signals,
    Ssr,
    ThirdPartyRenderer,
    Warnings,
    WasmSplit,
    Web,
}
impl std::fmt::Display for Feature {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Feature::Asset => std::fmt::Display::fmt("asset", f),
            Feature::CliConfig => std::fmt::Display::fmt("cli-config", f),
            Feature::Default => std::fmt::Display::fmt("default", f),
            Feature::Desktop => std::fmt::Display::fmt("desktop", f),
            Feature::Devtools => std::fmt::Display::fmt("devtools", f),
            Feature::Document => std::fmt::Display::fmt("document", f),
            Feature::FileEngine => std::fmt::Display::fmt("file_engine", f),
            Feature::Fullstack => std::fmt::Display::fmt("fullstack", f),
            Feature::Hooks => std::fmt::Display::fmt("hooks", f),
            Feature::Html => std::fmt::Display::fmt("html", f),
            Feature::Launch => std::fmt::Display::fmt("launch", f),
            Feature::Lib => std::fmt::Display::fmt("lib", f),
            Feature::Liveview => std::fmt::Display::fmt("liveview", f),
            Feature::Logger => std::fmt::Display::fmt("logger", f),
            Feature::Macro => std::fmt::Display::fmt("macro", f),
            Feature::Minimal => std::fmt::Display::fmt("minimal", f),
            Feature::Mobile => std::fmt::Display::fmt("mobile", f),
            Feature::Mounted => std::fmt::Display::fmt("mounted", f),
            Feature::Native => std::fmt::Display::fmt("native", f),
            Feature::Router => std::fmt::Display::fmt("router", f),
            Feature::Server => std::fmt::Display::fmt("server", f),
            Feature::Signals => std::fmt::Display::fmt("signals", f),
            Feature::Ssr => std::fmt::Display::fmt("ssr", f),
            Feature::ThirdPartyRenderer => std::fmt::Display::fmt("third-party-renderer", f),
            Feature::Warnings => std::fmt::Display::fmt("warnings", f),
            Feature::WasmSplit => std::fmt::Display::fmt("wasm-split", f),
            Feature::Web => std::fmt::Display::fmt("web", f),
        }
    }
}
pub static ENABLED_FEATURES: &[Feature] = &[
    #[cfg(feature = "asset")]
    Feature::Asset,
    #[cfg(feature = "cli-config")]
    Feature::CliConfig,
    #[cfg(feature = "default")]
    Feature::Default,
    #[cfg(feature = "desktop")]
    Feature::Desktop,
    #[cfg(feature = "devtools")]
    Feature::Devtools,
    #[cfg(feature = "document")]
    Feature::Document,
    #[cfg(feature = "file_engine")]
    Feature::FileEngine,
    #[cfg(feature = "fullstack")]
    Feature::Fullstack,
    #[cfg(feature = "hooks")]
    Feature::Hooks,
    #[cfg(feature = "html")]
    Feature::Html,
    #[cfg(feature = "launch")]
    Feature::Launch,
    #[cfg(feature = "lib")]
    Feature::Lib,
    #[cfg(feature = "liveview")]
    Feature::Liveview,
    #[cfg(feature = "logger")]
    Feature::Logger,
    #[cfg(feature = "macro")]
    Feature::Macro,
    #[cfg(feature = "minimal")]
    Feature::Minimal,
    #[cfg(feature = "mobile")]
    Feature::Mobile,
    #[cfg(feature = "mounted")]
    Feature::Mounted,
    #[cfg(feature = "native")]
    Feature::Native,
    #[cfg(feature = "router")]
    Feature::Router,
    #[cfg(feature = "server")]
    Feature::Server,
    #[cfg(feature = "signals")]
    Feature::Signals,
    #[cfg(feature = "ssr")]
    Feature::Ssr,
    #[cfg(feature = "third-party-renderer")]
    Feature::ThirdPartyRenderer,
    #[cfg(feature = "warnings")]
    Feature::Warnings,
    #[cfg(feature = "wasm-split")]
    Feature::WasmSplit,
    #[cfg(feature = "web")]
    Feature::Web,
];
use wasm_bindgen::prelude::*;
#[wasm_bindgen(inline_js = "export function show_features(features) {
            let pre = document.createElement('pre');
            pre.setAttribute('id', 'features');
            pre.innerText = features.join('\\n');
            document.body.appendChild(pre);
        }")]
extern "C" {
    fn show_features(features: Vec<String>);
}


pub fn launch() {
    let features_string: Vec<_> = ENABLED_FEATURES.iter().map(|f| f.to_string()).collect();
    #[cfg(target_arch = "wasm32")]
    show_features(features_string);
    #[cfg(not(target_arch = "wasm32"))]
    std::fs::write("features.txt", features_string.join("\n")).unwrap();
}
