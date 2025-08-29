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
            if let Err(err) = webbrowser::open(options.url.as_str()) {
                tracing::error!("Failed to open URL: {}", err);
            }
        }
    }
}
