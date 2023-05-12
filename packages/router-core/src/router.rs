use crate::history::HistoryProvider;
use std::str::FromStr;

#[derive(Debug, PartialEq)]
struct RouteParseError<E: std::fmt::Display> {
    attempted_routes: Vec<E>,
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

// #[derive(Props, PartialEq)]
// struct RouterProps {
//     current_route: String,
// }

trait Routable: FromStr + std::fmt::Display + Clone
where
    <Self as FromStr>::Err: std::fmt::Display,
{
//     fn render(self, cx: &ScopeState) -> Element;

//     fn comp(cx: Scope<RouterProps>) -> Element
//     where
//         Self: 'static,
//     {
//         let router = Self::from_str(&cx.props.current_route);
//         match router {
//             Ok(router) => router.render(cx),
//             Err(err) => {
//                 render! {pre {
//                     "{err}"
//                 }}
//             }
//         }
//     }
}

#[derive(Routable, Clone, Debug, PartialEq)]
enum Route {
    #[route("/(dynamic)")]
    Route1 { dynamic: String },
    #[route("/hello_world")]
    Route2 {},
    #[redirect("/(dynamic)/hello_world")]
    #[route("/hello_world/(dynamic)")]
    Route3 { dynamic: u32 },
    #[route("/(number1)/(number2)")]
    Route4 { number1: u32, number2: u32 },
    #[route("/")]
    Route5 {},
}

#[test]
fn display_works() {
    let route = Route::Route1 {
        dynamic: "hello".to_string(),
    };

    assert_eq!(route.to_string(), "/hello");

    let route = Route::Route3 { dynamic: 1234 };

    assert_eq!(route.to_string(), "/hello_world/1234");

    let route = Route::Route1 {
        dynamic: "hello_world2".to_string(),
    };

    assert_eq!(route.to_string(), "/hello_world2");
}

#[test]
fn from_string_works() {
    let w = "/hello";
    assert_eq!(
        Route::from_str(w),
        Ok(Route::Route1 {
            dynamic: "hello".to_string()
        })
    );
    let w = "/hello/";
    assert_eq!(
        Route::from_str(w),
        Ok(Route::Route1 {
            dynamic: "hello".to_string()
        })
    );

    let w = "/hello_world/1234";
    assert_eq!(Route::from_str(w), Ok(Route::Route3 { dynamic: 1234 }));
    let w = "/hello_world/1234/";
    assert_eq!(Route::from_str(w), Ok(Route::Route3 { dynamic: 1234 }));

    let w = "/hello_world2";
    assert_eq!(
        Route::from_str(w),
        Ok(Route::Route1 {
            dynamic: "hello_world2".to_string()
        })
    );

    let w = "/hello_world/-1";
    match Route::from_str(w) {
        Ok(r) => panic!("should not parse {r:?}"),
        Err(err) => println!("{err}"),
    }
}

#[test]
fn round_trip() {
    // Route1
    let string = "hello_world2";
    let route = Route::Route1 {
        dynamic: string.to_string(),
    };
    assert_eq!(Route::from_str(&route.to_string()), Ok(route));

    // Route2
    for num in 0..100 {
        let route = Route::Route3 { dynamic: num };
        assert_eq!(Route::from_str(&route.to_string()), Ok(route));
    }

    // Route3
    for num1 in 0..100 {
        for num2 in 0..100 {
            let route = Route::Route4 {
                number1: num1,
                number2: num2,
            };
            assert_eq!(Route::from_str(&route.to_string()), Ok(route));
        }
    }
}
