use crate::history::HistoryProvider;
use dioxus::prelude::*;

use std::str::FromStr;

#[derive(Debug, PartialEq)]
pub struct RouteParseError<E: std::fmt::Display> {
    pub attempted_routes: Vec<E>,
}

impl<E: std::fmt::Display> std::fmt::Display for RouteParseError<E> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Route did not match:\nAttempted Matches:\n")?;
        for (i, route) in self.attempted_routes.iter().enumerate() {
            writeln!(f, "{}) {route}", i + 1)?;
        }
        Ok(())
    }
}

struct Router<R: Routable, H: HistoryProvider>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    history: H,
    route: R,
}

impl<R: Routable, H: HistoryProvider> Router<R, H>
where
    <R as FromStr>::Err: std::fmt::Display,
{
    fn new(history: H) -> Result<Self, R::Err> {
        let path = history.current_path();
        Ok(Self {
            history,
            route: R::from_str(path.as_str())?,
        })
    }
}

pub trait FromQuery {
    fn from_query(query: &str) -> Self;
}

impl<T: for<'a> From<&'a str>> FromQuery for T {
    fn from_query(query: &str) -> Self {
        T::from(query)
    }
}

pub trait FromRouteSegment: Sized {
    type Err;

    fn from_route_segment(route: &str) -> Result<Self, Self::Err>;
}

impl<T: FromStr> FromRouteSegment for T
where
    <T as FromStr>::Err: std::fmt::Display,
{
    type Err = <T as FromStr>::Err;

    fn from_route_segment(route: &str) -> Result<Self, Self::Err> {
        T::from_str(route)
    }
}

pub trait ToRouteSegments {
    fn display_route_segements(self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result;
}

impl<I, T: std::fmt::Display> ToRouteSegments for I
where
    I: IntoIterator<Item = T>,
{
    fn display_route_segements(self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for segment in self {
            write!(f, "/")?;
            write!(f, "{}", segment)?;
        }
        Ok(())
    }
}

pub trait FromRouteSegments: Sized {
    type Err;

    fn from_route_segments(segments: &[&str]) -> Result<Self, Self::Err>;
}

impl<I: std::iter::FromIterator<String>> FromRouteSegments for I {
    type Err = <String as FromRouteSegment>::Err;

    fn from_route_segments(segments: &[&str]) -> Result<Self, Self::Err> {
        segments
            .iter()
            .map(|s| String::from_route_segment(s))
            .collect()
    }
}

#[derive(Props, PartialEq)]
pub struct RouterProps {
    pub current_route: String,
}

pub trait Routable: FromStr + std::fmt::Display + Clone
where
    <Self as FromStr>::Err: std::fmt::Display,
{
    fn render(self, cx: &ScopeState) -> Element;

    fn comp(cx: Scope<RouterProps>) -> Element
    where
        Self: 'static,
    {
        let router = Self::from_str(&cx.props.current_route);
        match router {
            Ok(router) => router.render(cx),
            Err(err) => {
                render! {
                    pre {
                        "{err}"
                    }
                }
            }
        }
    }
}
