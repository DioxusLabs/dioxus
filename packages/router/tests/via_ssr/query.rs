#[derive(Debug, Clone, PartialEq)]
enum Query {
    Id(u64),
    Color(String),
}

impl From<&str> for Query {
    fn from(query: &str) -> Self {
        // e.g. split query on `&` and split pairs on `=`
        Query::Id(10)
    }
}

impl Display for Query {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "id=10")
    }
}

#[component]
fn Index(query: Query) -> Element {
    rsx! {
        h1 { "Index" }
    }
}

#[test]
fn test_query_parsing() {
    let route = Route::Index {
        query: Query::Id(10),
    };
    assert_eq!(route.to_string(), "/?id=10");
    let parsed_route = "/?id=10".parse::<Route>().unwrap();
    assert_eq!(parsed_route, route);
}

#[derive(Debug, Clone, PartialEq, Routable)]
enum Route {
    #[route("/?:..query")]
    Index { query: Query },
}
