#![allow(non_snake_case)]
use crate::history::HistoryProvider;
use dioxus::prelude::*;

use std::{cell::RefCell, rc::Rc, str::FromStr, sync::Arc};

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

#[derive(Clone)]
pub struct Router {
    subscribers: Rc<RefCell<Vec<ScopeId>>>,
    update_any: Arc<dyn Fn(ScopeId)>,
    history: Rc<dyn HistoryProvider>,
    route: Rc<RefCell<Option<Rc<dyn RouteRenderable>>>>,
}

impl Router {
    fn set_route<R: Routable + 'static>(&self, route: R)
    where
        R::Err: std::fmt::Display,
    {
        *self.route.borrow_mut() = Some(Rc::new(route));
        for subscriber in self.subscribers.borrow().iter() {
            (self.update_any)(*subscriber);
        }
    }
}

fn use_router(cx: &ScopeState) -> &Router {
    use_context(cx).unwrap()
}

fn use_route(cx: &ScopeState) -> Rc<dyn RouteRenderable> {
    let router = use_router(cx);
    cx.use_hook(|| {
        router.subscribers.borrow_mut().push(cx.scope_id());
    });
    router.route.borrow().clone().unwrap()
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
pub trait Routable: std::fmt::Display + std::str::FromStr + 'static
where
    <Self as FromStr>::Err: std::fmt::Display,
{
    fn render<'a>(&self, cx: &'a ScopeState, level: usize) -> Element<'a>;
}

trait RoutableFactory {
    type Err: std::fmt::Display;
    type Routable: Routable + FromStr<Err = Self::Err>;
}

impl<R: Routable + FromStr> RoutableFactory for R
where
    <R as FromStr>::Err: std::fmt::Display,
{
    type Err = <R as FromStr>::Err;
    type Routable = R;
}

trait RouteRenderable: std::fmt::Display + 'static {
    fn render<'a>(&self, cx: &'a ScopeState, level: usize) -> Element<'a>;
}

impl<R: Routable> RouteRenderable for R
where
    <R as FromStr>::Err: std::fmt::Display,
{
    fn render<'a>(&self, cx: &'a ScopeState, level: usize) -> Element<'a> {
        self.render(cx, level)
    }
}

#[derive(Clone)]
struct OutletContext {
    current_level: usize,
}

fn use_outlet_context(cx: &ScopeState) -> &OutletContext {
    let outlet_context = use_context(cx).unwrap();
    outlet_context
}

impl OutletContext {
    fn render(cx: &ScopeState) -> Element<'_> {
        let outlet = use_outlet_context(cx);
        let current_level = outlet.current_level;
        cx.provide_context({
            OutletContext {
                current_level: current_level + 1,
            }
        });

        use_route(cx).render(cx, current_level)
    }
}

pub fn Outlet(cx: Scope) -> Element {
    OutletContext::render(cx)
}

pub fn Router<R: Routable, H: HistoryProvider + Default + 'static>(
    cx: Scope<RouterProps>,
) -> Element
where
    <R as FromStr>::Err: std::fmt::Display,
{
    let current_route = R::from_str(&cx.props.current_route);
    let router = use_context_provider(cx, || Router {
        subscribers: Rc::default(),
        update_any: cx.schedule_update_any(),
        history: Rc::<H>::default(),
        route: Rc::new(RefCell::new(None)),
    });

    use_context_provider(cx, || OutletContext { current_level: 1 });

    match current_route {
        Ok(current_route) => {
            router.set_route(current_route);

            router.route.borrow().as_ref().unwrap().render(cx, 0)
        }
        Err(err) => {
            render! {
                pre {
                    "{err}"
                }
            }
        }
    }
}
