use serde::{de::DeserializeOwned, Serialize};

/// TODO: Document this
pub fn server_cached<O: 'static + Serialize + DeserializeOwned>(server_fn: impl Fn() -> O) -> O {
    #[cfg(feature = "ssr")]
    {
        let data =
            crate::html_storage::deserialize::take_server_data().unwrap_or_else(|| server_fn());
        let sc = crate::prelude::server_context();
        sc.push_html_data(&data);
        data
    }
    #[cfg(not(feature = "ssr"))]
    {
        let data = server_fn();
        data
    }
}
