use std::collections::{BTreeMap, HashSet};

use crate::test_routes_segment;

#[test]
fn without_params() {
    let expected = vec![
        "/",
        "/external-navigation-failure/",
        "/named-navigation-failure/",
        "/redirect/",
        "/test/",
        "/test/nest/",
        "/test/nest/double-nest/",
        "/test/\\parameter/",
        "/\\matching-parameter/",
    ];

    assert_eq!(expected, test_routes_segment().sitemap());
}

#[test]
fn with_params() {
    let expected: HashSet<String> = vec![
        "/",
        "/external-navigation-failure/",
        "/named-navigation-failure/",
        "/redirect/",
        "/test/",
        "/test/nest/",
        "/test/nest/double-nest/",
        "/test/some-test-value/",
        "/test/some-other-value/",
        "/some-other-matching-value/",
    ]
    .into_iter()
    .map(|path| path.to_string())
    .collect();

    let params = {
        let mut params = BTreeMap::new();

        params.insert("parameter", {
            let mut p = HashSet::new();
            p.insert(String::from("some-test-value"));
            p.insert(String::from("some-other-value"));
            p
        });

        params.insert("matching-parameter", {
            let mut p = HashSet::new();
            p.insert(String::from("some-matching-value"));
            p.insert(String::from("some-other-matching-value"));
            p.insert(String::from("some-other-matching-value")); // duplicate intended
            p
        });

        params
    };

    assert_eq!(
        expected,
        test_routes_segment().sitemap_with_parameters(&params)
    );
}
