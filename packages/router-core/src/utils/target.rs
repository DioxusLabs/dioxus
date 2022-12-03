use either::Either;

use crate::{
    navigation::{NavigationTarget, Query},
    segments::NameMap,
    Name,
};

use super::resolve_name;

pub fn resolve_target(
    names: &NameMap,
    target: &NavigationTarget,
) -> Either<Either<String, Name>, String> {
    match target {
        NavigationTarget::Internal(i) => Either::Left(Either::Left(i.clone())),
        NavigationTarget::Named {
            name,
            parameters,
            query,
        } => Either::Left(
            resolve_name(names, name, parameters)
                .map(|mut p| {
                    if let Some(q) = query {
                        match q {
                            Query::Single(s) => {
                                if !s.starts_with('?') {
                                    p += "?";
                                }
                                p += &s;
                            }
                            #[cfg(feature = "serde")]
                            Query::List(l) => {
                                let res = serde_urlencoded::to_string(l);
                                // TODO: find a test case where this assertion is not met
                                debug_assert!(res.is_ok(), "cannot serialize query list: {l:?}");
                                if let Ok(q) = res {
                                    p += "?";
                                    p += &q;
                                }
                            }
                        }
                    }

                    p
                })
                .map(|p| Either::Left(p))
                .unwrap_or(Either::Right(name.clone())),
        ),
        NavigationTarget::External(e) => Either::Right(e.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use crate::{prelude::RootIndex, routes::Segment, segments::NamedSegment};

    use super::*;

    #[test]
    fn resolve_internal() {
        let names = NamedSegment::from_segment(&Segment::<&str>::empty());
        assert_eq!(
            resolve_target(&names, &NavigationTarget::Internal("/test".to_string())),
            Either::Left(Either::Left(String::from("/test")))
        );
    }

    #[test]
    fn resolve_named() {
        let names = NamedSegment::from_segment(&Segment::<&str>::empty());
        assert_eq!(
            resolve_target(&names, &NavigationTarget::named::<RootIndex>()),
            Either::Left(Either::Left(String::from("/")))
        );
    }

    #[test]
    fn resolve_named_with_query_single() {
        let names = NamedSegment::from_segment(&Segment::<&str>::empty());
        let without = resolve_target(
            &names,
            &NavigationTarget::named::<RootIndex>().query("huhu"),
        );
        let with = resolve_target(
            &names,
            &NavigationTarget::named::<RootIndex>().query("?huhu"),
        );
        let correct = Either::Left(Either::Left(String::from("/?huhu")));
        assert_eq!(with, correct);
        assert_eq!(without, correct);
        assert_eq!(with, without);
    }

    #[test]
    #[cfg(feature = "serde")]
    fn resolve_named_with_query_list() {
        let names = NamedSegment::from_segment(&Segment::<&str>::empty());
        assert_eq!(
            resolve_target(
                &names,
                &NavigationTarget::named::<RootIndex>()
                    .query(vec![("some", "test"), ("another", "value")])
            ),
            Either::Left(Either::Left(String::from("/?some=test&another=value")))
        );
    }

    #[test]
    fn resolve_external() {
        let names = NamedSegment::from_segment(&Segment::<&str>::empty());
        assert_eq!(
            resolve_target(
                &names,
                &NavigationTarget::External("https://dioxuslabs.com/".to_string())
            ),
            Either::Right(String::from("https://dioxuslabs.com/"))
        );
    }
}
