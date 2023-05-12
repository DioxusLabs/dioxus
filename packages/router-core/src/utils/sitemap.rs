use std::collections::BTreeMap;

use urlencoding::encode;


pub fn gen_sitemap<T: Clone>(seg: &Segment<T>, current: &str, map: &mut Vec<String>) {
    for (p, r) in &seg.fixed {
        let current = format!("{current}/{p}");
        map.push(current.clone());
        if let Some(n) = &r.nested {
            gen_sitemap(n, &current, map);
        }
    }

    for (_, r) in &seg.matching {
        let current = format!("{current}/\\{}", r.key);
        map.push(current.clone());
        if let Some(n) = &r.nested {
            gen_sitemap(n, &current, map);
        }
    }

    if let Some(r) = &seg.catch_all {
        let current = format!("{current}/\\{}", r.key);
        map.push(current.clone());
        if let Some(n) = &r.nested {
            gen_sitemap(n, &current, map)
        }
    }
}

pub fn gen_parameter_sitemap<T: Clone>(
    seg: &Segment<T>,
    parameters: &BTreeMap<Name, Vec<String>>,
    current: &str,
    map: &mut Vec<String>,
) {
    for (p, r) in &seg.fixed {
        let current = format!("{current}/{p}");
        map.push(current.clone());
        if let Some(n) = &r.nested {
            gen_parameter_sitemap(n, parameters, &current, map);
        }
    }

    for (m, r) in &seg.matching {
        if let Some(rp) = parameters.get(&r.key) {
            for p in rp {
                if m.matches(p) {
                    let current = format!("{current}/{}", encode(p).into_owned());
                    map.push(current.clone());
                    if let Some(n) = &r.nested {
                        gen_parameter_sitemap(n, parameters, &current, map);
                    }
                }
            }
        }
    }

    if let Some(r) = &seg.catch_all {
        if let Some(rp) = parameters.get(&r.key) {
            for p in rp {
                let current = format!("{current}/{}", encode(p).into_owned());
                map.push(current.clone());
                if let Some(n) = &r.nested {
                    gen_parameter_sitemap(n, parameters, &current, map);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::routes::{ParameterRoute, Route};

    use super::*;

    fn test_segment() -> Segment<&'static str> {
        Segment::empty()
            .fixed(
                "fixed",
                Route::empty().nested(Segment::empty().fixed("nested", Route::empty())),
            )
            .matching(
                String::from("m1"),
                ParameterRoute::empty::<u8>().nested(
                    Segment::empty().matching(String::from("n2"), ParameterRoute::empty::<u16>()),
                ),
            )
            .matching(String::from("no match"), ParameterRoute::empty::<u32>())
            .matching(String::from("no parameter"), ParameterRoute::empty::<u64>())
            .catch_all(
                ParameterRoute::empty::<u32>()
                    .nested(Segment::empty().catch_all(ParameterRoute::empty::<u16>())),
            )
    }

    #[test]
    fn sitemap() {
        let mut result = Vec::new();
        result.push(String::from("/"));
        gen_sitemap(&test_segment(), "", &mut result);

        assert_eq!(
            result,
            vec![
                "/",
                "/fixed",
                "/fixed/nested",
                "/\\u8",
                "/\\u8/\\u16",
                "/\\u32",
                "/\\u64",
                "/\\u32",
                "/\\u32/\\u16"
            ]
        );
    }

    #[test]
    fn sitemap_with_parameters() {
        let mut parameters = BTreeMap::new();
        parameters.insert(Name::of::<u8>(), vec!["m1".to_string(), "m2".to_string()]);
        parameters.insert(Name::of::<u16>(), vec!["n1".to_string(), "n2".to_string()]);
        parameters.insert(Name::of::<u32>(), vec!["catch all".to_string()]);

        let mut result = Vec::new();
        result.push(String::from("/"));
        gen_parameter_sitemap(&test_segment(), &parameters, "", &mut result);

        assert_eq!(
            result,
            vec![
                "/",
                "/fixed",
                "/fixed/nested",
                "/m1",
                "/m1/n2",
                "/catch%20all",
                "/catch%20all/n1",
                "/catch%20all/n2"
            ]
        );
    }
}
