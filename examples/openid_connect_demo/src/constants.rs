pub const DIOXUS_FRONT_AUTH_TOKEN: &str = "auth_token";
pub const DIOXUS_FRONT_AUTH_REQUEST: &str = "auth_request";

pub const DIOXUS_FRONT_ISSUER_URL: &str = match option_env!("DIOXUS_FRONT_ISSUER_URL") {
    None => "https://samples.auth0.com/authorize",
    Some(v) => v,
};

pub const DIOXUS_FRONT_CLIENT_ID: &str = match option_env!("DIOXUS_FRONT_CLIENT_ID") {
    None => "kbyuFDidLLm280LIwVFiazOqjO3ty8KH",
    Some(v) => v,
};

pub const DIOXUS_FRONT_URL: &str = match option_env!("DIOXUS_FRONT_URL") {
    None => "localhost:8080",
    Some(v) => v,
};
