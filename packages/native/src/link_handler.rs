use blitz_traits::{
    navigation::{NavigationOptions, NavigationProvider},
    net::Method,
};

pub(crate) struct DioxusNativeNavigationProvider;

impl NavigationProvider for DioxusNativeNavigationProvider {
    fn navigate_to(&self, options: NavigationOptions) {
        if options.method == Method::GET
            && matches!(options.url.scheme(), "http" | "https" | "mailto")
        {
            if let Err(_err) = webbrowser::open(options.url.as_str()) {
                #[cfg(feature = "tracing")]
                tracing::error!("Failed to open URL: {}", _err);
            }
        }
    }
}
