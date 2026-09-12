//! Runtime support for `#[derive(Routable)]`.
//!
//! The derive used to emit one error enum per route variant, nest and redirect plus the code to
//! walk the url segments with cloned iterators at every branch. Everything that does not depend
//! on the user's types lives here instead, so the generated code is a thin match tree over
//! [`RouteSegments`] that reports failures through the shared [`RouteMatchError`].

use std::borrow::Cow;
use std::fmt::{self, Display, Write};
use std::str::FromStr;

use crate::query_sets::{FRAGMENT_ASCII_SET, PATH_ASCII_SET, QUERY_ASCII_SET};
use crate::routable::{FromRouteSegment, FromRouteSegments};

/// The route variant, redirect or nest a parse failure is reported against.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RouteMatchSite {
    /// Name of the (private) match error type the derive used to emit, kept for `Debug` output.
    pub error_type: &'static str,
    /// `"Route"`, `"Redirect"` or `"Nest"`.
    pub kind: &'static str,
    /// The variant name, or the generated `RedirectNParseError` / `NestNParseError` name.
    pub name: &'static str,
    /// The route string as written in the attribute.
    pub route: &'static str,
}

impl RouteMatchSite {
    /// The url had more segments than the route.
    pub fn extra_segments(&'static self, trailing: String) -> RouteMatchError {
        self.error(SegmentParseError::ExtraSegments(trailing))
    }

    /// A static segment did not match.
    pub fn static_segment(&'static self, expected: &'static str, found: &str) -> RouteMatchError {
        self.error(SegmentParseError::StaticSegment {
            expected,
            found: found.to_string(),
        })
    }

    /// A child route failed to parse.
    pub fn child_route(&'static self, error: impl Display) -> RouteMatchError {
        self.error(SegmentParseError::ChildRoute(error.to_string()))
    }

    fn error(&'static self, error: SegmentParseError) -> RouteMatchError {
        RouteMatchError { site: self, error }
    }
}

/// One failed attempt at matching a url against a route.
pub struct RouteMatchError {
    site: &'static RouteMatchSite,
    error: SegmentParseError,
}

impl Display for RouteMatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let RouteMatchSite {
            kind, name, route, ..
        } = self.site;
        write!(
            f,
            "{kind} '{name}' ('{route}') did not match:\n{}",
            self.error
        )
    }
}

impl fmt::Debug for RouteMatchError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}({})", self.site.error_type, self)
    }
}

/// Why one segment of a route did not match.
pub enum SegmentParseError {
    /// The url had more segments than the route.
    ExtraSegments(String),
    /// The child route failed to parse.
    ChildRoute(String),
    /// A static segment was different.
    StaticSegment {
        /// The segment in the route.
        expected: &'static str,
        /// The segment in the url.
        found: String,
    },
    /// A dynamic segment failed to parse.
    DynamicSegment {
        /// The field name.
        name: &'static str,
        /// The field type.
        ty: &'static str,
        /// The parse error.
        error: String,
    },
    /// The url ended before a dynamic segment.
    MissingDynamicSegment {
        /// The field name.
        name: &'static str,
        /// The field type.
        ty: &'static str,
    },
    /// A catch-all segment failed to parse.
    CatchAll {
        /// The field name.
        name: &'static str,
        /// The field type.
        ty: &'static str,
        /// The parse error.
        error: String,
    },
}

impl Display for SegmentParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ExtraSegments(segments) => {
                write!(f, "Found additional trailing segments: {segments}")
            }
            Self::ChildRoute(error) => f.write_str(error),
            Self::StaticSegment { expected, found } => write!(
                f,
                "Static segment '{expected}' did not match instead found '{found}'"
            ),
            Self::DynamicSegment { name, ty, error } => {
                write!(f, "Dynamic segment '({name}:{ty})' did not match: {error}")
            }
            Self::MissingDynamicSegment { name, ty } => {
                write!(f, "Dynamic segment '({name}:{ty})' was missing")
            }
            Self::CatchAll { name, ty, error } => {
                write!(
                    f,
                    "Catch-all segment '({name}:{ty})' did not match: {error}"
                )
            }
        }
    }
}

/// The percent-decoded path segments of a url, without the leading `/`.
#[doc(hidden)]
pub struct RouteSegments<'a>(Vec<Cow<'a, str>>);

impl<'a> RouteSegments<'a> {
    /// Split `route` (the url without query or hash) into decoded segments.
    ///
    /// Returns `None` when the url does not start with `/`. A trailing `/` is ignored so that
    /// `/path/` and `/path` parse the same way.
    pub fn parse(route: &'a str) -> Option<Self> {
        if !route.starts_with('/') {
            return None;
        }
        let route = route.strip_suffix('/').unwrap_or(route);
        let segments = route
            .split('/')
            .skip(1)
            .map(|s| {
                percent_encoding::percent_decode_str(s)
                    .decode_utf8()
                    .unwrap_or(s.into())
            })
            .collect();
        Some(Self(segments))
    }

    /// The segment at `index`, if the url is that long.
    pub fn get(&self, index: usize) -> Option<&str> {
        self.0.get(index).map(|s| &**s)
    }

    /// The number of segments in the url.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether the url has no segments.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Whether the url has exactly `count` segments.
    pub fn ends_at(&self, count: usize) -> bool {
        self.0.len() == count
    }

    /// The segments from `from` onwards joined with `/`, for [`SegmentParseError::ExtraSegments`].
    pub fn trailing(&self, from: usize) -> String {
        self.0[from..].join("/")
    }

    /// The url a child router sees: the segments from `from` onwards plus the raw query and hash.
    pub fn child_route(&self, from: usize, raw_query: &str, raw_hash: &str) -> String {
        let mut route = String::from("/");
        for segment in &self.0[from..] {
            route.push_str(segment);
            route.push('/');
        }
        if !raw_query.is_empty() {
            route.push('?');
            route.push_str(raw_query);
        }
        if !raw_hash.is_empty() {
            route.push('#');
            route.push_str(raw_hash);
        }
        route
    }

    /// Parse the segment at `index` as the dynamic field `name: ty`.
    pub fn dynamic<T: FromRouteSegment>(
        &self,
        index: usize,
        site: &'static RouteMatchSite,
        name: &'static str,
        ty: &'static str,
    ) -> Result<T, RouteMatchError>
    where
        T::Err: Display,
    {
        match self.get(index) {
            Some(segment) => T::from_route_segment(segment).map_err(|error| {
                site.error(SegmentParseError::DynamicSegment {
                    name,
                    ty,
                    error: error.to_string(),
                })
            }),
            None => Err(site.error(SegmentParseError::MissingDynamicSegment { name, ty })),
        }
    }

    /// Parse the segments from `from` onwards as the catch-all field `name: ty`.
    pub fn catch_all<T: FromRouteSegments>(
        &self,
        from: usize,
        site: &'static RouteMatchSite,
        name: &'static str,
        ty: &'static str,
    ) -> Result<T, RouteMatchError> {
        let segments: Vec<&str> = self.0[from..].iter().map(|s| &**s).collect();
        T::from_route_segments(&segments).map_err(|error| {
            site.error(SegmentParseError::CatchAll {
                name,
                ty,
                error: error.to_string(),
            })
        })
    }

    /// Parse the segments from `from` onwards (plus the raw query and hash) as a child route.
    pub fn child<T: FromStr>(
        &self,
        from: usize,
        raw_query: &str,
        raw_hash: &str,
        site: &'static RouteMatchSite,
    ) -> Result<T, RouteMatchError>
    where
        T::Err: Display,
    {
        T::from_str(&self.child_route(from, raw_query, raw_hash))
            .map_err(|error| site.child_route(error))
    }
}

/// Write `/` followed by the percent-encoded display of a dynamic segment.
#[doc(hidden)]
pub fn write_path_segment<W: Write + ?Sized>(f: &mut W, segment: &dyn Display) -> fmt::Result {
    let segment = segment.to_string();
    write!(
        f,
        "/{}",
        percent_encoding::utf8_percent_encode(&segment, PATH_ASCII_SET)
    )
}

/// Write `?` followed by the percent-encoded display of a whole query.
#[doc(hidden)]
pub fn write_query<W: Write + ?Sized>(f: &mut W, query: &dyn Display) -> fmt::Result {
    let query = query.to_string();
    write!(
        f,
        "?{}",
        percent_encoding::utf8_percent_encode(&query, QUERY_ASCII_SET)
    )
}

/// Write one `name=value` query argument, followed by `&` unless it is empty or the last one.
#[doc(hidden)]
pub fn write_query_argument<W: Write + ?Sized>(
    f: &mut W,
    argument: &dyn Display,
    last: bool,
) -> fmt::Result {
    let argument = argument.to_string();
    write!(
        f,
        "{}",
        percent_encoding::utf8_percent_encode(&argument, QUERY_ASCII_SET)
    )?;
    if !last && !argument.is_empty() {
        f.write_char('&')?;
    }
    Ok(())
}

/// Write `#` followed by the percent-encoded display of a hash fragment, unless it is empty.
#[doc(hidden)]
pub fn write_hash_fragment<W: Write + ?Sized>(f: &mut W, hash: &dyn Display) -> fmt::Result {
    let hash = hash.to_string();
    if hash.is_empty() {
        return Ok(());
    }
    write!(
        f,
        "#{}",
        percent_encoding::utf8_percent_encode(&hash, FRAGMENT_ASCII_SET)
    )
}
