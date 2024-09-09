use crate::Route;
use dioxus::prelude::*;
use dioxus_router::prelude::*;
use std::{cell::RefCell, fmt};

thread_local! {
    static PREFIX: RefCell<&'static str> = RefCell::new("");
}

pub fn use_prefix(prefix: Option<&'static str>) {
    use_effect(use_reactive(&prefix, move |_| {
        if let Some(prefix) = prefix {
            PREFIX.try_with(|cell| *cell.borrow_mut() = prefix).unwrap();
        }
    }));
}

#[derive(Clone, PartialEq)]
pub struct PrefixedRoute(pub(crate) Route);

pub struct PrefixError;

impl fmt::Display for PrefixError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("DummyError")
    }
}

impl std::str::FromStr for PrefixedRoute {
    type Err = PrefixError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let prefix = &*PREFIX.try_with(|cell| *cell.borrow()).unwrap();

        if s.is_empty() || s.starts_with(prefix) {
            let route = s[prefix.len()..]
                .parse::<Route>()
                .map_err(|_| PrefixError)?;
            Ok(PrefixedRoute(route))
        } else {
            Err(PrefixError)
        }
    }
}

impl fmt::Display for PrefixedRoute {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let prefix = &*PREFIX.try_with(|cell| *cell.borrow()).unwrap();
        f.write_str(&prefix)?;
        self.0.fmt(f)
    }
}

impl Routable for PrefixedRoute {
    const SITE_MAP: &'static [SiteMapSegment] = &[];

    fn render(&self, level: usize) -> Element {
        self.0.render(level)
    }

    fn static_routes() -> Vec<Self> {
        Route::static_routes()
            .into_iter()
            .map(PrefixedRoute)
            .collect()
    }
}
