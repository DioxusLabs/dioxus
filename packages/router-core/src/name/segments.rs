use std::collections::BTreeMap;

use crate::{
    prelude::RootIndex,
    routes::{ParameterRoute, Route, Segment},
    Name,
};

pub type NameMap = BTreeMap<Name, Vec<NamedSegment>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NamedSegment {
    Fixed(String),
    Parameter(Name),
}

impl NamedSegment {
    pub fn from_segment<T: Clone>(segment: &Segment<T>) -> NameMap {
        let mut res = BTreeMap::new();
        res.insert(Name::of::<RootIndex>(), vec![]);
        Self::from_segment_inner(Vec::new(), segment, &mut res);
        res
    }

    fn from_segment_inner<T: Clone>(
        current: Vec<NamedSegment>,
        segment: &Segment<T>,
        result: &mut NameMap,
    ) {
        for (p, r) in &segment.fixed {
            Self::from_route(current.clone(), p, r, result);
        }

        for (_, r) in &segment.matching {
            Self::from_parameter_route(current.clone(), r, result);
        }

        if let Some(r) = &segment.catch_all {
            Self::from_parameter_route(current, r, result);
        }
    }

    fn from_route<T: Clone>(
        mut current: Vec<NamedSegment>,
        path: &str,
        route: &Route<T>,
        result: &mut NameMap,
    ) {
        current.push(Self::Fixed(path.to_string()));

        if let Some(n) = &route.name {
            debug_assert!(!result.contains_key(n), "duplicate name: {n}");
            result.entry(n.clone()).or_insert_with(|| current.clone());
        }

        if let Some(n) = &route.nested {
            Self::from_segment_inner(current, n, result);
        }
    }

    fn from_parameter_route<T: Clone>(
        mut current: Vec<NamedSegment>,
        route: &ParameterRoute<T>,
        result: &mut NameMap,
    ) {
        current.push(Self::Parameter(route.key.clone()));

        if let Some(n) = &route.name {
            debug_assert!(!result.contains_key(n), "duplicate name: {n}");
            result.entry(n.clone()).or_insert_with(|| current.clone());
        }

        if let Some(n) = &route.nested {
            Self::from_segment_inner(current, n, result);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{NamedSegment::*, *};

    #[test]
    fn create_map() {
        assert_eq!(
            NamedSegment::from_segment(
                &Segment::<&'static str>::empty()
                    .fixed(
                        "fixed",
                        Route::empty()
                            .name::<u32>()
                            .nested(Segment::empty().fixed("nested", Route::empty().name::<u64>()))
                    )
                    .matching(
                        String::from(""),
                        ParameterRoute::empty::<i32>().name::<i32>().nested(
                            Segment::empty().matching(
                                String::from(""),
                                ParameterRoute::empty::<i64>().name::<i64>()
                            )
                        )
                    )
                    .catch_all(ParameterRoute::empty::<f32>().name::<f32>().nested(
                        Segment::empty().catch_all(ParameterRoute::empty::<f64>().name::<f64>())
                    ))
            ),
            {
                let mut r = BTreeMap::new();
                r.insert(Name::of::<RootIndex>(), vec![]);

                r.insert(Name::of::<u32>(), vec![Fixed(String::from("fixed"))]);
                r.insert(
                    Name::of::<u64>(),
                    vec![Fixed(String::from("fixed")), Fixed(String::from("nested"))],
                );

                r.insert(Name::of::<i32>(), vec![Parameter(Name::of::<i32>())]);
                r.insert(
                    Name::of::<i64>(),
                    vec![Parameter(Name::of::<i32>()), Parameter(Name::of::<i64>())],
                );

                r.insert(Name::of::<f32>(), vec![Parameter(Name::of::<f32>())]);
                r.insert(
                    Name::of::<f64>(),
                    vec![Parameter(Name::of::<f32>()), Parameter(Name::of::<f64>())],
                );

                r
            }
        )
    }
}
