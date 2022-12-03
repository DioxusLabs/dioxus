use std::collections::HashMap;

use urlencoding::encode;

use crate::{
    segments::{NameMap, NamedSegment},
    Name,
};

pub fn resolve_name(
    map: &NameMap,
    name: &Name,
    parameters: &HashMap<Name, String>,
) -> Option<String> {
    debug_assert!(
        map.contains_key(&name),
        "named navigation to unknown name: {name}"
    );
    let target = map.get(&name)?;

    let mut res = String::new();
    for t in target {
        res += "/";
        match t {
            NamedSegment::Fixed(f) => res += f,
            NamedSegment::Parameter(p) => {
                debug_assert!(
                    parameters.contains_key(p),
                    "named navigation is missing parameter: target {name} parameter {p}"
                );
                let val = parameters.get(p)?;

                res += &encode(val);
            }
        }
    }

    if res.is_empty() {
        res += "/";
    }

    Some(res)
}

#[cfg(test)]
mod tests {
    use crate::{
        prelude::RootIndex,
        routes::{ParameterRoute, Route, Segment},
    };

    use super::*;

    fn test_map() -> NameMap {
        NamedSegment::from_segment(
            &Segment::<&str>::empty()
                .fixed(
                    "fixed",
                    Route::empty().name::<u8>().nested(
                        Segment::empty().catch_all(ParameterRoute::empty::<u16>().name::<u32>()),
                    ),
                )
                .catch_all(ParameterRoute::empty::<i8>().name::<i16>()),
        )
    }

    #[test]
    fn root_index() {
        assert_eq!(
            resolve_name(&test_map(), &Name::of::<RootIndex>(), &HashMap::new()),
            Some(String::from("/"))
        )
    }

    #[test]
    fn fixed() {
        assert_eq!(
            resolve_name(&test_map(), &Name::of::<u8>(), &HashMap::new()),
            Some(String::from("/fixed"))
        )
    }

    #[test]
    fn matching() {
        assert_eq!(
            resolve_name(&test_map(), &Name::of::<i16>(), &{
                let mut r = HashMap::new();
                r.insert(Name::of::<i8>(), String::from("test"));
                r
            }),
            Some(String::from("/test"))
        );
    }

    #[test]
    fn nested() {
        assert_eq!(
            resolve_name(&test_map(), &Name::of::<u32>(), &{
                let mut r = HashMap::new();
                r.insert(Name::of::<u16>(), String::from("nested"));
                r
            }),
            Some(String::from("/fixed/nested"))
        );
    }

    #[test]
    #[should_panic = "named navigation to unknown name: bool"]
    #[cfg(debug_assertions)]
    fn missing_name_debug() {
        resolve_name(&test_map(), &Name::of::<bool>(), &HashMap::new());
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn missing_name_release() {
        assert!(resolve_name(&test_map(), &Name::of::<bool>(), &HashMap::new()).is_none());
    }

    #[test]
    #[should_panic = "named navigation is missing parameter: target u32 parameter u16"]
    #[cfg(debug_assertions)]
    fn missing_parameter_debug() {
        resolve_name(&test_map(), &Name::of::<u32>(), &HashMap::new());
    }

    #[test]
    #[cfg(not(debug_assertions))]
    fn missing_parameter_release() {
        assert!(resolve_name(&test_map(), &Name::of::<u32>(), &HashMap::new()).is_none());
    }

    #[test]
    fn url_encoding() {
        assert_eq!(
            resolve_name(&test_map(), &Name::of::<u32>(), &{
                let mut r = HashMap::new();
                r.insert(Name::of::<u16>(), String::from("🥳"));
                r
            }),
            Some(String::from("/fixed/%F0%9F%A5%B3"))
        );
    }
}
