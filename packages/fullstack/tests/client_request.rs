use dioxus_fullstack::{ClientRequest, set_server_url};
use http::Method;
use serde::Serialize;

#[test]
fn client_requests_preserve_server_base_paths() {
    #[derive(Serialize)]
    struct Query {
        search: &'static str,
    }

    for (server_url, expected_url) in [
        ("", "http://this.is.not.a.real.url:9000/api/echo"),
        (
            "https://example.com:8443",
            "https://example.com:8443/api/echo",
        ),
        (
            "https://example.com:8443/foo",
            "https://example.com:8443/foo/api/echo",
        ),
        ("/foo", "http://this.is.not.a.real.url:9000/foo/api/echo"),
        (
            "/foo/bar",
            "http://this.is.not.a.real.url:9000/foo/bar/api/echo",
        ),
        ("/", "http://this.is.not.a.real.url:9000/api/echo"),
        ("/foo/", "http://this.is.not.a.real.url:9000/foo/api/echo"),
    ] {
        set_server_url(server_url);
        let request = ClientRequest::new(Method::GET, "/api/echo".into(), &());
        assert_eq!(request.url().as_str(), expected_url);
        assert_eq!(request.method(), Method::GET);

        let request =
            ClientRequest::new(Method::POST, "/api/echo".into(), &Query { search: "a&b" });
        assert_eq!(
            request.url().as_str(),
            format!("{expected_url}?search=a%26b")
        );
        assert_eq!(request.method(), Method::POST);
    }
}
