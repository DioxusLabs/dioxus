use std::str::FromStr;

use url::Url;

pub struct ParsedRoute {
    pub(crate) url: url::Url,
}

impl ParsedRoute {
    pub(crate) fn new(url: Url) -> Self {
        Self { url }
    }

    // get the underlying url
    pub fn url(&self) -> &Url {
        &self.url
    }

    pub fn query(&self) -> Option<&String> {
        None
    }

    /// Returns the nth segment in the path. Paths that end with a slash have
    /// the slash removed before determining the segments. If the path has
    /// fewer segments than `n` then this method returns `None`.
    pub fn nth_segment(&self, n: usize) -> Option<&str> {
        self.url.path_segments()?.nth(n)
    }

    /// Returns the last segment in the path. Paths that end with a slash have
    /// the slash removed before determining the segments. The root path, `/`,
    /// will return an empty string.
    pub fn last_segment(&self) -> Option<&str> {
        self.url.path_segments()?.last()
    }

    /// Get the named parameter from the path, as defined in your router. The
    /// value will be parsed into the type specified by `T` by calling
    /// `value.parse::<T>()`. This method returns `None` if the named
    /// parameter does not exist in the current path.
    pub fn segment<T>(&self, name: &str) -> Option<&str>
    where
        T: FromStr,
    {
        self.url.path_segments()?.find(|&f| f.eq(name))
    }

    /// Get the named parameter from the path, as defined in your router. The
    /// value will be parsed into the type specified by `T` by calling
    /// `value.parse::<T>()`. This method returns `None` if the named
    /// parameter does not exist in the current path.
    pub fn parse_segment<T>(&self, name: &str) -> Option<Result<T, T::Err>>
    where
        T: FromStr,
    {
        self.url
            .path_segments()?
            .find(|&f| f.eq(name))
            .map(|f| f.parse::<T>())
    }
}

#[test]
fn parses_location() {
    let route = ParsedRoute::new(Url::parse("app:///foo/bar?baz=qux&quux=corge").unwrap());
}
