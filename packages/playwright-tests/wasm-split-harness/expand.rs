#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use dioxus::prelude::*;
use futures::AsyncReadExt;
use std::pin::Pin;
use wasm_bindgen::prelude::*;
fn main() {
    dioxus::launch(app);
    dioxus::launch(|| {
        dioxus_core::Element::Ok({
            #[cfg(debug_assertions)]
            fn __original_template() -> &'static dioxus_core::internal::HotReloadedTemplate {
                static __ORIGINAL_TEMPLATE: ::std::sync::OnceLock<
                    dioxus_core::internal::HotReloadedTemplate,
                > = ::std::sync::OnceLock::new();
                if __ORIGINAL_TEMPLATE.get().is_none() {
                    _ = __ORIGINAL_TEMPLATE.set(dioxus_core::internal::HotReloadedTemplate::new(
                        None,
                        <[_]>::into_vec(::alloc::boxed::box_new([
                            dioxus_core::internal::HotReloadDynamicNode::Dynamic(0usize),
                        ])),
                        ::alloc::vec::Vec::new(),
                        ::alloc::vec::Vec::new(),
                        __TEMPLATE_ROOTS,
                    ));
                }
                __ORIGINAL_TEMPLATE.get().unwrap()
            }
            #[cfg(debug_assertions)]
            let __template_read = {
                static __NORMALIZED_FILE: &'static str = {
                    const PATH: &str = ::const_format::pmr::__AssertStr {
                        x: {
                            const ARGS_OSRCTFL4A: ::const_format::__str_methods::ReplaceInput =
                                ::const_format::__str_methods::ReplaceInputConv(
                                    "packages/playwright-tests/wasm-split-harness/src/main.rs",
                                    "\\\\",
                                    "/",
                                )
                                .conv();
                            {
                                const OB: &[::const_format::pmr::u8;
                                     ARGS_OSRCTFL4A.replace_length()] = &ARGS_OSRCTFL4A.replace();
                                const OS: &::const_format::pmr::str = unsafe {
                                    {
                                        let bytes: &'static [::const_format::pmr::u8] = OB;
                                        let string: &'static ::const_format::pmr::str = {
                                            ::const_format::__hidden_utils::PtrToRef {
                                                ptr: bytes as *const [::const_format::pmr::u8]
                                                    as *const str,
                                            }
                                            .reff
                                        };
                                        string
                                    }
                                };
                                OS
                            }
                        },
                    }
                    .x;
                    ::const_format::pmr::__AssertStr {
                        x: {
                            const ARGS_OSRCTFL4A: ::const_format::__str_methods::ReplaceInput =
                                ::const_format::__str_methods::ReplaceInputConv(PATH, '\\', "/")
                                    .conv();
                            {
                                const OB: &[::const_format::pmr::u8;
                                     ARGS_OSRCTFL4A.replace_length()] = &ARGS_OSRCTFL4A.replace();
                                const OS: &::const_format::pmr::str = unsafe {
                                    {
                                        let bytes: &'static [::const_format::pmr::u8] = OB;
                                        let string: &'static ::const_format::pmr::str = {
                                            ::const_format::__hidden_utils::PtrToRef {
                                                ptr: bytes as *const [::const_format::pmr::u8]
                                                    as *const str,
                                            }
                                            .reff
                                        };
                                        string
                                    }
                                };
                                OS
                            }
                        },
                    }
                    .x
                };
                static __TEMPLATE: GlobalSignal<
                    Option<dioxus_core::internal::HotReloadedTemplate>,
                > = GlobalSignal::with_location(
                    || None::<dioxus_core::internal::HotReloadedTemplate>,
                    __NORMALIZED_FILE,
                    9u32,
                    9u32,
                    0usize,
                );
                dioxus_core::Runtime::current()
                    .ok()
                    .map(|_| __TEMPLATE.read())
            };
            #[cfg(debug_assertions)]
            let __template_read = match __template_read
                .as_ref()
                .map(|__template_read| __template_read.as_ref())
            {
                Some(Some(__template_read)) => &__template_read,
                _ => __original_template(),
            };
            #[cfg(debug_assertions)]
            let mut __dynamic_literal_pool =
                dioxus_core::internal::DynamicLiteralPool::new(::alloc::vec::Vec::new());
            let __dynamic_nodes: [dioxus_core::DynamicNode; 1usize] =
                [dioxus_core::DynamicNode::Component({
                    use dioxus_core::prelude::Properties;
                    let __comp = ({ fc_to_builder(Router::<Route>).build() })
                        .into_vcomponent(Router::<Route>);
                    __comp
                })];
            let __dynamic_attributes: [Box<[dioxus_core::Attribute]>; 0usize] = [];
            #[doc(hidden)]
            static __TEMPLATE_ROOTS: &[dioxus_core::TemplateNode] =
                &[dioxus_core::TemplateNode::Dynamic { id: 0usize }];
            #[cfg(debug_assertions)]
            {
                let mut __dynamic_value_pool = dioxus_core::internal::DynamicValuePool::new(
                    Vec::from(__dynamic_nodes),
                    Vec::from(__dynamic_attributes),
                    __dynamic_literal_pool,
                );
                __dynamic_value_pool.render_with(__template_read)
            }
        })
    });
}
enum Route {
    #[layout(Nav)]
    #[route("/home")]
    Home,
    #[route("/about")]
    About,
}
/// An error that can occur when trying to parse the route variant `/home`.
#[allow(non_camel_case_types)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub enum HomeParseError {
    /// An error that can occur when extra segments are provided after the route.
    ExtraSegments(String),
    /// An error that can occur when trying to parse the static segment '/home'.
    StaticSegment0ParseError(String),
}
impl std::fmt::Debug for HomeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{0}({1})", "HomeParseError", self))
    }
}
impl std::fmt::Display for HomeParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExtraSegments(segments) => f.write_fmt(format_args!(
                "Found additional trailing segments: {0}",
                segments
            ))?,
            Self::StaticSegment0ParseError(found) => f.write_fmt(format_args!(
                "Static segment \'{0}\' did not match instead found \'{1}\'",
                "home", found
            ))?,
        }
        Ok(())
    }
}
/// An error that can occur when trying to parse the route variant `/about`.
#[allow(non_camel_case_types)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub enum AboutParseError {
    /// An error that can occur when extra segments are provided after the route.
    ExtraSegments(String),
    /// An error that can occur when trying to parse the static segment '/about'.
    StaticSegment0ParseError(String),
}
impl std::fmt::Debug for AboutParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{0}({1})", "AboutParseError", self))
    }
}
impl std::fmt::Display for AboutParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExtraSegments(segments) => f.write_fmt(format_args!(
                "Found additional trailing segments: {0}",
                segments
            ))?,
            Self::StaticSegment0ParseError(found) => f.write_fmt(format_args!(
                "Static segment \'{0}\' did not match instead found \'{1}\'",
                "about", found
            ))?,
        }
        Ok(())
    }
}
/// An error that can occur when trying to parse the route enum [`Route`].
#[allow(non_camel_case_types)]
#[allow(clippy::derive_partial_eq_without_eq)]
pub enum RouteMatchError {
    /// An error that can occur when trying to parse the route [`Route::Home`] ('/home').
    Home(HomeParseError),
    /// An error that can occur when trying to parse the route [`Route::About`] ('/about').
    About(AboutParseError),
}
impl std::fmt::Debug for RouteMatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{0}({1})", "RouteMatchError", self))
    }
}
impl std::fmt::Display for RouteMatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Home(err) => f.write_fmt(format_args!(
                "Route \'{0}\' (\'{1}\') did not match:\n{2}",
                "Home", "/home", err
            ))?,
            Self::About(err) => f.write_fmt(format_args!(
                "Route \'{0}\' (\'{1}\') did not match:\n{2}",
                "About", "/about", err
            ))?,
        }
        Ok(())
    }
}
impl std::fmt::Display for Route {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        #[allow(unused)]
        match self {
            Self::Home {} => {
                f.write_fmt(format_args!("/{0}", "home"))?;
            }
            Self::About {} => {
                f.write_fmt(format_args!("/{0}", "about"))?;
            }
        }
        Ok(())
    }
}
impl dioxus_router::routable::Routable for Route
where
    Self: Clone,
{
    const SITE_MAP: &'static [dioxus_router::routable::SiteMapSegment] = &[
        dioxus_router::routable::SiteMapSegment {
            segment_type: dioxus_router::routable::SegmentType::Static("home"),
            children: &[],
        },
        dioxus_router::routable::SiteMapSegment {
            segment_type: dioxus_router::routable::SegmentType::Static("about"),
            children: &[],
        },
    ];
    fn render(&self, level: usize) -> dioxus_core::Element {
        let myself = self.clone();
        match (level, myself) {
            #[allow(unused)]
            (0usize, Self::Home { .. }) => {
                dioxus_core::Element::Ok({
                    #[cfg(debug_assertions)]
                    fn __original_template() -> &'static dioxus_core::internal::HotReloadedTemplate
                    {
                        static __ORIGINAL_TEMPLATE: ::std::sync::OnceLock<
                            dioxus_core::internal::HotReloadedTemplate,
                        > = ::std::sync::OnceLock::new();
                        if __ORIGINAL_TEMPLATE.get().is_none() {
                            _ = __ORIGINAL_TEMPLATE.set(
                                dioxus_core::internal::HotReloadedTemplate::new(
                                    None,
                                    <[_]>::into_vec(::alloc::boxed::box_new([
                                        dioxus_core::internal::HotReloadDynamicNode::Dynamic(
                                            0usize,
                                        ),
                                    ])),
                                    ::alloc::vec::Vec::new(),
                                    ::alloc::vec::Vec::new(),
                                    __TEMPLATE_ROOTS,
                                ),
                            );
                        }
                        __ORIGINAL_TEMPLATE.get().unwrap()
                    }
                    #[cfg(debug_assertions)]
                    let __template_read = {
                        static __NORMALIZED_FILE: &'static str = {
                            const PATH : & str = :: const_format :: pmr :: __AssertStr { x : { const ARGS_OSRCTFL4A : :: const_format :: __str_methods :: ReplaceInput = :: const_format :: __str_methods :: ReplaceInputConv ("packages/playwright-tests/wasm-split-harness/src/main.rs" , "\\\\" , "/") . conv () ; { const OB : & [:: const_format :: pmr :: u8 ; ARGS_OSRCTFL4A . replace_length ()] = & ARGS_OSRCTFL4A . replace () ; const OS : & :: const_format :: pmr :: str = unsafe { { let bytes : & 'static [:: const_format :: pmr :: u8] = OB ; let string : & 'static :: const_format :: pmr :: str = { :: const_format :: __hidden_utils :: PtrToRef { ptr : bytes as * const [:: const_format :: pmr :: u8] as * const str , } . reff } ; string } } ; OS } } , } . x ;
                            ::const_format::pmr::__AssertStr {
                                x: {
                                    const ARGS_OSRCTFL4A:
                                        ::const_format::__str_methods::ReplaceInput =
                                        ::const_format::__str_methods::ReplaceInputConv(
                                            PATH, '\\', "/",
                                        )
                                        .conv();
                                    {
                                        const OB: &[::const_format::pmr::u8;
                                             ARGS_OSRCTFL4A.replace_length()] =
                                            &ARGS_OSRCTFL4A.replace();
                                        const OS: &::const_format::pmr::str = unsafe {
                                            {
                                                let bytes: &'static [::const_format::pmr::u8] = OB;
                                                let string: &'static ::const_format::pmr::str = {
                                                    ::const_format::__hidden_utils::PtrToRef {
                                                        ptr: bytes
                                                            as *const [::const_format::pmr::u8]
                                                            as *const str,
                                                    }
                                                    .reff
                                                };
                                                string
                                            }
                                        };
                                        OS
                                    }
                                },
                            }
                            .x
                        };
                        static __TEMPLATE: GlobalSignal<
                            Option<dioxus_core::internal::HotReloadedTemplate>,
                        > = GlobalSignal::with_location(
                            || None::<dioxus_core::internal::HotReloadedTemplate>,
                            __NORMALIZED_FILE,
                            15u32,
                            10u32,
                            0usize,
                        );
                        dioxus_core::Runtime::current()
                            .ok()
                            .map(|_| __TEMPLATE.read())
                    };
                    #[cfg(debug_assertions)]
                    let __template_read = match __template_read
                        .as_ref()
                        .map(|__template_read| __template_read.as_ref())
                    {
                        Some(Some(__template_read)) => &__template_read,
                        _ => __original_template(),
                    };
                    #[cfg(debug_assertions)]
                    let mut __dynamic_literal_pool =
                        dioxus_core::internal::DynamicLiteralPool::new(::alloc::vec::Vec::new());
                    let __dynamic_nodes: [dioxus_core::DynamicNode; 1usize] =
                        [dioxus_core::DynamicNode::Component({
                            use dioxus_core::prelude::Properties;
                            let __comp = ({ fc_to_builder(Nav).build() }).into_vcomponent(Nav);
                            __comp
                        })];
                    let __dynamic_attributes: [Box<[dioxus_core::Attribute]>; 0usize] = [];
                    #[doc(hidden)]
                    static __TEMPLATE_ROOTS: &[dioxus_core::TemplateNode] =
                        &[dioxus_core::TemplateNode::Dynamic { id: 0usize }];
                    #[cfg(debug_assertions)]
                    {
                        let mut __dynamic_value_pool = dioxus_core::internal::DynamicValuePool::new(
                            Vec::from(__dynamic_nodes),
                            Vec::from(__dynamic_attributes),
                            __dynamic_literal_pool,
                        );
                        __dynamic_value_pool.render_with(__template_read)
                    }
                })
            }
            #[allow(unused)]
            (1usize, Self::Home {}) => {
                #[no_mangle]
                fn routeHome(args: Route) -> Element {
                    match args {
                        Route::Home {} => Home(),
                        _ => ::core::panicking::panic("internal error: entered unreachable code"),
                    }
                }
                static MODULE: ::dioxus::wasm_split::LazyLoader<Route, Element> = {
                    #[link(wasm_import_module = "./__wasm_split.js")]
                    extern "C" {
                        #[no_mangle]
                        fn __wasm_split_load_moduleHome(
                            callback: unsafe extern "C" fn(*const ::std::ffi::c_void, bool),
                            data: *const ::std::ffi::c_void,
                        ) -> ();
                        #[allow(improper_ctypes)]
                        #[no_mangle]
                        fn __wasm_split_00moduleHome00_import_488533529a102174fdd216581dd5a8ae_routeHome(
                            arg: Route,
                        ) -> Element;
                    }
                    #[allow(improper_ctypes_definitions)]
                    #[no_mangle]
                    pub extern "C" fn __wasm_split_00moduleHome00_export_488533529a102174fdd216581dd5a8ae_routeHome(
                        arg: Route,
                    ) -> Element {
                        routeHome(arg)
                    }
                    const __wasm_split_loader_moduleHome: ::std::thread::LocalKey<
                        ::wasm_split::LazySplitLoader,
                    > = {
                        #[inline]
                        fn __init() -> ::wasm_split::LazySplitLoader {
                            unsafe {
                                ::wasm_split::LazySplitLoader::new(__wasm_split_load_moduleHome)
                            }
                        }
                        unsafe {
                            use ::std::thread::LocalKey;
                            use ::std::thread::local_impl::LazyStorage;
                            LocalKey::new(|init| {
                                static VAL: LazyStorage<::wasm_split::LazySplitLoader> =
                                    LazyStorage::new();
                                VAL.get(init, __init)
                            })
                        }
                    };;
                    :: wasm_split :: LazyLoader { key : & __wasm_split_loader_moduleHome , imported : __wasm_split_00moduleHome00_import_488533529a102174fdd216581dd5a8ae_routeHome , }
                };
                fn LoaderInner(args: Route) -> Element {
                    use_resource(|| async move { MODULE.load().await }).suspend()?;
                    MODULE.call(args).unwrap()
                }
                LoaderInner(Route::Home {})
            }
            #[allow(unused)]
            (0usize, Self::About { .. }) => {
                dioxus_core::Element::Ok({
                    #[cfg(debug_assertions)]
                    fn __original_template() -> &'static dioxus_core::internal::HotReloadedTemplate
                    {
                        static __ORIGINAL_TEMPLATE: ::std::sync::OnceLock<
                            dioxus_core::internal::HotReloadedTemplate,
                        > = ::std::sync::OnceLock::new();
                        if __ORIGINAL_TEMPLATE.get().is_none() {
                            _ = __ORIGINAL_TEMPLATE.set(
                                dioxus_core::internal::HotReloadedTemplate::new(
                                    None,
                                    <[_]>::into_vec(::alloc::boxed::box_new([
                                        dioxus_core::internal::HotReloadDynamicNode::Dynamic(
                                            0usize,
                                        ),
                                    ])),
                                    ::alloc::vec::Vec::new(),
                                    ::alloc::vec::Vec::new(),
                                    __TEMPLATE_ROOTS,
                                ),
                            );
                        }
                        __ORIGINAL_TEMPLATE.get().unwrap()
                    }
                    #[cfg(debug_assertions)]
                    let __template_read = {
                        static __NORMALIZED_FILE: &'static str = {
                            const PATH : & str = :: const_format :: pmr :: __AssertStr { x : { const ARGS_OSRCTFL4A : :: const_format :: __str_methods :: ReplaceInput = :: const_format :: __str_methods :: ReplaceInputConv ("packages/playwright-tests/wasm-split-harness/src/main.rs" , "\\\\" , "/") . conv () ; { const OB : & [:: const_format :: pmr :: u8 ; ARGS_OSRCTFL4A . replace_length ()] = & ARGS_OSRCTFL4A . replace () ; const OS : & :: const_format :: pmr :: str = unsafe { { let bytes : & 'static [:: const_format :: pmr :: u8] = OB ; let string : & 'static :: const_format :: pmr :: str = { :: const_format :: __hidden_utils :: PtrToRef { ptr : bytes as * const [:: const_format :: pmr :: u8] as * const str , } . reff } ; string } } ; OS } } , } . x ;
                            ::const_format::pmr::__AssertStr {
                                x: {
                                    const ARGS_OSRCTFL4A:
                                        ::const_format::__str_methods::ReplaceInput =
                                        ::const_format::__str_methods::ReplaceInputConv(
                                            PATH, '\\', "/",
                                        )
                                        .conv();
                                    {
                                        const OB: &[::const_format::pmr::u8;
                                             ARGS_OSRCTFL4A.replace_length()] =
                                            &ARGS_OSRCTFL4A.replace();
                                        const OS: &::const_format::pmr::str = unsafe {
                                            {
                                                let bytes: &'static [::const_format::pmr::u8] = OB;
                                                let string: &'static ::const_format::pmr::str = {
                                                    ::const_format::__hidden_utils::PtrToRef {
                                                        ptr: bytes
                                                            as *const [::const_format::pmr::u8]
                                                            as *const str,
                                                    }
                                                    .reff
                                                };
                                                string
                                            }
                                        };
                                        OS
                                    }
                                },
                            }
                            .x
                        };
                        static __TEMPLATE: GlobalSignal<
                            Option<dioxus_core::internal::HotReloadedTemplate>,
                        > = GlobalSignal::with_location(
                            || None::<dioxus_core::internal::HotReloadedTemplate>,
                            __NORMALIZED_FILE,
                            15u32,
                            10u32,
                            0usize,
                        );
                        dioxus_core::Runtime::current()
                            .ok()
                            .map(|_| __TEMPLATE.read())
                    };
                    #[cfg(debug_assertions)]
                    let __template_read = match __template_read
                        .as_ref()
                        .map(|__template_read| __template_read.as_ref())
                    {
                        Some(Some(__template_read)) => &__template_read,
                        _ => __original_template(),
                    };
                    #[cfg(debug_assertions)]
                    let mut __dynamic_literal_pool =
                        dioxus_core::internal::DynamicLiteralPool::new(::alloc::vec::Vec::new());
                    let __dynamic_nodes: [dioxus_core::DynamicNode; 1usize] =
                        [dioxus_core::DynamicNode::Component({
                            use dioxus_core::prelude::Properties;
                            let __comp = ({ fc_to_builder(Nav).build() }).into_vcomponent(Nav);
                            __comp
                        })];
                    let __dynamic_attributes: [Box<[dioxus_core::Attribute]>; 0usize] = [];
                    #[doc(hidden)]
                    static __TEMPLATE_ROOTS: &[dioxus_core::TemplateNode] =
                        &[dioxus_core::TemplateNode::Dynamic { id: 0usize }];
                    #[cfg(debug_assertions)]
                    {
                        let mut __dynamic_value_pool = dioxus_core::internal::DynamicValuePool::new(
                            Vec::from(__dynamic_nodes),
                            Vec::from(__dynamic_attributes),
                            __dynamic_literal_pool,
                        );
                        __dynamic_value_pool.render_with(__template_read)
                    }
                })
            }
            #[allow(unused)]
            (1usize, Self::About {}) => {
                #[no_mangle]
                fn routeAbout(args: Route) -> Element {
                    match args {
                        Route::About {} => About(),
                        _ => ::core::panicking::panic("internal error: entered unreachable code"),
                    }
                }
                static MODULE: ::dioxus::wasm_split::LazyLoader<Route, Element> = {
                    #[link(wasm_import_module = "./__wasm_split.js")]
                    extern "C" {
                        #[no_mangle]
                        fn __wasm_split_load_moduleAbout(
                            callback: unsafe extern "C" fn(*const ::std::ffi::c_void, bool),
                            data: *const ::std::ffi::c_void,
                        ) -> ();
                        #[allow(improper_ctypes)]
                        #[no_mangle]
                        fn __wasm_split_00moduleAbout00_import_a71257f083b8689b8ab18e7f3f27b3e6_routeAbout(
                            arg: Route,
                        ) -> Element;
                    }
                    #[allow(improper_ctypes_definitions)]
                    #[no_mangle]
                    pub extern "C" fn __wasm_split_00moduleAbout00_export_a71257f083b8689b8ab18e7f3f27b3e6_routeAbout(
                        arg: Route,
                    ) -> Element {
                        routeAbout(arg)
                    }
                    const __wasm_split_loader_moduleAbout: ::std::thread::LocalKey<
                        ::wasm_split::LazySplitLoader,
                    > = {
                        #[inline]
                        fn __init() -> ::wasm_split::LazySplitLoader {
                            unsafe {
                                ::wasm_split::LazySplitLoader::new(__wasm_split_load_moduleAbout)
                            }
                        }
                        unsafe {
                            use ::std::thread::LocalKey;
                            use ::std::thread::local_impl::LazyStorage;
                            LocalKey::new(|init| {
                                static VAL: LazyStorage<::wasm_split::LazySplitLoader> =
                                    LazyStorage::new();
                                VAL.get(init, __init)
                            })
                        }
                    };;
                    :: wasm_split :: LazyLoader { key : & __wasm_split_loader_moduleAbout , imported : __wasm_split_00moduleAbout00_import_a71257f083b8689b8ab18e7f3f27b3e6_routeAbout , }
                };
                fn LoaderInner(args: Route) -> Element {
                    use_resource(|| async move { MODULE.load().await }).suspend()?;
                    MODULE.call(args).unwrap()
                }
                LoaderInner(Route::About {})
            }
            _ => VNode::empty(),
        }
    }
}
impl<'a> core::convert::TryFrom<&'a str> for Route {
    type Error = <Self as std::str::FromStr>::Err;
    fn try_from(s: &'a str) -> ::std::result::Result<Self, Self::Error> {
        s.parse()
    }
}
impl std::str::FromStr for Route {
    type Err = dioxus_router::routable::RouteParseError<RouteMatchError>;
    fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
        let route = s;
        let (route, hash) = route.split_once('#').unwrap_or((route, ""));
        let (route, query) = route.split_once('?').unwrap_or((route, ""));
        let route = route.strip_suffix('/').unwrap_or(route);
        let query = dioxus_router::exports::urlencoding::decode(query).unwrap_or(query.into());
        let hash = dioxus_router::exports::urlencoding::decode(hash).unwrap_or(hash.into());
        let mut segments = route
            .split('/')
            .map(|s| dioxus_router::exports::urlencoding::decode(s).unwrap_or(s.into()));
        if s.starts_with('/') {
            let _ = segments.next();
        } else {
            return Err(dioxus_router::routable::RouteParseError {
                attempted_routes: Vec::new(),
            });
        }
        let mut errors = Vec::new();
        {
            let mut segments = segments.clone();
            let segment = segments.next();
            if let Some(segment) = segment.as_deref() {
                if "home" == segment {
                    let remaining_segments = segments.clone();
                    let mut segments_clone = segments.clone();
                    let next_segment = segments_clone.next();
                    if next_segment.is_none() {
                        return Ok(Route::Home {});
                    } else {
                        let mut trailing = String::new();
                        for seg in remaining_segments {
                            trailing += &*seg;
                            trailing += "/";
                        }
                        trailing.pop();
                        errors.push(RouteMatchError::Home(HomeParseError::ExtraSegments(
                            trailing,
                        )))
                    }
                } else {
                    errors.push(RouteMatchError::Home(
                        HomeParseError::StaticSegment0ParseError(segment.to_string()),
                    ))
                }
            }
        }
        {
            let mut segments = segments.clone();
            let segment = segments.next();
            if let Some(segment) = segment.as_deref() {
                if "about" == segment {
                    let remaining_segments = segments.clone();
                    let mut segments_clone = segments.clone();
                    let next_segment = segments_clone.next();
                    if next_segment.is_none() {
                        return Ok(Route::About {});
                    } else {
                        let mut trailing = String::new();
                        for seg in remaining_segments {
                            trailing += &*seg;
                            trailing += "/";
                        }
                        trailing.pop();
                        errors.push(RouteMatchError::About(AboutParseError::ExtraSegments(
                            trailing,
                        )))
                    }
                } else {
                    errors.push(RouteMatchError::About(
                        AboutParseError::StaticSegment0ParseError(segment.to_string()),
                    ))
                }
            }
        }
        Err(dioxus_router::routable::RouteParseError {
            attempted_routes: errors,
        })
    }
}
#[automatically_derived]
impl ::core::clone::Clone for Route {
    #[inline]
    fn clone(&self) -> Route {
        match self {
            Route::Home => Route::Home,
            Route::About => Route::About,
        }
    }
}
#[automatically_derived]
impl ::core::marker::StructuralPartialEq for Route {}
#[automatically_derived]
impl ::core::cmp::PartialEq for Route {
    #[inline]
    fn eq(&self, other: &Route) -> bool {
        let __self_discr = ::core::intrinsics::discriminant_value(self);
        let __arg1_discr = ::core::intrinsics::discriminant_value(other);
        __self_discr == __arg1_discr
    }
}
#[automatically_derived]
impl ::core::fmt::Debug for Route {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(
            f,
            match self {
                Route::Home => "Home",
                Route::About => "About",
            },
        )
    }
}
#[allow(non_snake_case)]
fn Nav() -> Element {
    {
        dioxus_core::internal::verify_component_called_as_component(Nav);
        {
            dioxus_core::Element::Ok({
                #[cfg(debug_assertions)]
                fn __original_template() -> &'static dioxus_core::internal::HotReloadedTemplate {
                    static __ORIGINAL_TEMPLATE: ::std::sync::OnceLock<
                        dioxus_core::internal::HotReloadedTemplate,
                    > = ::std::sync::OnceLock::new();
                    if __ORIGINAL_TEMPLATE.get().is_none() {
                        _ = __ORIGINAL_TEMPLATE.set(
                            dioxus_core::internal::HotReloadedTemplate::new(
                                None,
                                <[_]>::into_vec(::alloc::boxed::box_new([
                                    dioxus_core::internal::HotReloadDynamicNode::Dynamic(0usize),
                                    dioxus_core::internal::HotReloadDynamicNode::Dynamic(1usize),
                                    dioxus_core::internal::HotReloadDynamicNode::Dynamic(2usize),
                                ])),
                                ::alloc::vec::Vec::new(),
                                ::alloc::vec::Vec::new(),
                                __TEMPLATE_ROOTS,
                            ),
                        );
                    }
                    __ORIGINAL_TEMPLATE.get().unwrap()
                }
                #[cfg(debug_assertions)]
                let __template_read = {
                    static __NORMALIZED_FILE: &'static str = {
                        const PATH: &str = ::const_format::pmr::__AssertStr {
                            x: {
                                const ARGS_OSRCTFL4A: ::const_format::__str_methods::ReplaceInput =
                                    ::const_format::__str_methods::ReplaceInputConv(
                                        "packages/playwright-tests/wasm-split-harness/src/main.rs",
                                        "\\\\",
                                        "/",
                                    )
                                    .conv();
                                {
                                    const OB: &[::const_format::pmr::u8;
                                         ARGS_OSRCTFL4A.replace_length()] =
                                        &ARGS_OSRCTFL4A.replace();
                                    const OS: &::const_format::pmr::str = unsafe {
                                        {
                                            let bytes: &'static [::const_format::pmr::u8] = OB;
                                            let string: &'static ::const_format::pmr::str = {
                                                ::const_format::__hidden_utils::PtrToRef {
                                                    ptr: bytes as *const [::const_format::pmr::u8]
                                                        as *const str,
                                                }
                                                .reff
                                            };
                                            string
                                        }
                                    };
                                    OS
                                }
                            },
                        }
                        .x;
                        ::const_format::pmr::__AssertStr {
                            x: {
                                const ARGS_OSRCTFL4A: ::const_format::__str_methods::ReplaceInput =
                                    ::const_format::__str_methods::ReplaceInputConv(PATH, '\\', "/")
                                        .conv();
                                {
                                    const OB: &[::const_format::pmr::u8;
                                         ARGS_OSRCTFL4A.replace_length()] =
                                        &ARGS_OSRCTFL4A.replace();
                                    const OS: &::const_format::pmr::str = unsafe {
                                        {
                                            let bytes: &'static [::const_format::pmr::u8] = OB;
                                            let string: &'static ::const_format::pmr::str = {
                                                ::const_format::__hidden_utils::PtrToRef {
                                                    ptr: bytes as *const [::const_format::pmr::u8]
                                                        as *const str,
                                                }
                                                .reff
                                            };
                                            string
                                        }
                                    };
                                    OS
                                }
                            },
                        }
                        .x
                    };
                    static __TEMPLATE: GlobalSignal<
                        Option<dioxus_core::internal::HotReloadedTemplate>,
                    > = GlobalSignal::with_location(
                        || None::<dioxus_core::internal::HotReloadedTemplate>,
                        __NORMALIZED_FILE,
                        27u32,
                        5u32,
                        0usize,
                    );
                    dioxus_core::Runtime::current()
                        .ok()
                        .map(|_| __TEMPLATE.read())
                };
                #[cfg(debug_assertions)]
                let __template_read = match __template_read
                    .as_ref()
                    .map(|__template_read| __template_read.as_ref())
                {
                    Some(Some(__template_read)) => &__template_read,
                    _ => __original_template(),
                };
                #[cfg(debug_assertions)]
                let mut __dynamic_literal_pool =
                    dioxus_core::internal::DynamicLiteralPool::new(::alloc::vec::Vec::new());
                let __dynamic_nodes: [dioxus_core::DynamicNode; 3usize] = [
                    dioxus_core::DynamicNode::Component({
                        use dioxus_core::prelude::Properties;
                        let __comp = ({ fc_to_builder (Link) . to (Route :: Home { }) . children ({ dioxus_core :: Element :: Ok ({ # [cfg (debug_assertions)] fn __original_template () -> & 'static dioxus_core :: internal :: HotReloadedTemplate { static __ORIGINAL_TEMPLATE : :: std :: sync :: OnceLock < dioxus_core :: internal :: HotReloadedTemplate > = :: std :: sync :: OnceLock :: new () ; if __ORIGINAL_TEMPLATE . get () . is_none () { _ = __ORIGINAL_TEMPLATE . set (dioxus_core :: internal :: HotReloadedTemplate :: new (None , :: alloc :: vec :: Vec :: new () , :: alloc :: vec :: Vec :: new () , :: alloc :: vec :: Vec :: new () , __TEMPLATE_ROOTS)) ; } __ORIGINAL_TEMPLATE . get () . unwrap () } # [cfg (debug_assertions)] let __template_read = { static __NORMALIZED_FILE : & 'static str = { const PATH : & str = :: const_format :: pmr :: __AssertStr { x : { const ARGS_OSRCTFL4A : :: const_format :: __str_methods :: ReplaceInput = :: const_format :: __str_methods :: ReplaceInputConv ("packages/playwright-tests/wasm-split-harness/src/main.rs" , "\\\\" , "/") . conv () ; { const OB : & [:: const_format :: pmr :: u8 ; ARGS_OSRCTFL4A . replace_length ()] = & ARGS_OSRCTFL4A . replace () ; const OS : & :: const_format :: pmr :: str = unsafe { { let bytes : & 'static [:: const_format :: pmr :: u8] = OB ; let string : & 'static :: const_format :: pmr :: str = { :: const_format :: __hidden_utils :: PtrToRef { ptr : bytes as * const [:: const_format :: pmr :: u8] as * const str , } . reff } ; string } } ; OS } } , } . x ; :: const_format :: pmr :: __AssertStr { x : { const ARGS_OSRCTFL4A : :: const_format :: __str_methods :: ReplaceInput = :: const_format :: __str_methods :: ReplaceInputConv (PATH , '\\' , "/") . conv () ; { const OB : & [:: const_format :: pmr :: u8 ; ARGS_OSRCTFL4A . replace_length ()] = & ARGS_OSRCTFL4A . replace () ; const OS : & :: const_format :: pmr :: str = unsafe { { let bytes : & 'static [:: const_format :: pmr :: u8] = OB ; let string : & 'static :: const_format :: pmr :: str = { :: const_format :: __hidden_utils :: PtrToRef { ptr : bytes as * const [:: const_format :: pmr :: u8] as * const str , } . reff } ; string } } ; OS } } , } . x } ; static __TEMPLATE : GlobalSignal < Option < dioxus_core :: internal :: HotReloadedTemplate > > = GlobalSignal :: with_location (| | None :: < dioxus_core :: internal :: HotReloadedTemplate > , __NORMALIZED_FILE , 27u32 , 5u32 , 1usize) ; dioxus_core :: Runtime :: current () . ok () . map (| _ | __TEMPLATE . read ()) } ; # [cfg (debug_assertions)] let __template_read = match __template_read . as_ref () . map (| __template_read | __template_read . as_ref ()) { Some (Some (__template_read)) => & __template_read , _ => __original_template () , } ; # [cfg (debug_assertions)] let mut __dynamic_literal_pool = dioxus_core :: internal :: DynamicLiteralPool :: new (:: alloc :: vec :: Vec :: new ()) ; let __dynamic_nodes : [dioxus_core :: DynamicNode ; 0usize] = [] ; let __dynamic_attributes : [Box < [dioxus_core :: Attribute] > ; 0usize] = [] ; # [doc (hidden)] static __TEMPLATE_ROOTS : & [dioxus_core :: TemplateNode] = & [dioxus_core :: TemplateNode :: Text { text : "Home" }] ; # [cfg (debug_assertions)] { let mut __dynamic_value_pool = dioxus_core :: internal :: DynamicValuePool :: new (Vec :: from (__dynamic_nodes) , Vec :: from (__dynamic_attributes) , __dynamic_literal_pool) ; __dynamic_value_pool . render_with (__template_read) } }) }) . build () }) . into_vcomponent (Link) ;
                        __comp
                    }),
                    dioxus_core::DynamicNode::Component({
                        use dioxus_core::prelude::Properties;
                        let __comp = ({ fc_to_builder (Link) . to (Route :: About { }) . children ({ dioxus_core :: Element :: Ok ({ # [cfg (debug_assertions)] fn __original_template () -> & 'static dioxus_core :: internal :: HotReloadedTemplate { static __ORIGINAL_TEMPLATE : :: std :: sync :: OnceLock < dioxus_core :: internal :: HotReloadedTemplate > = :: std :: sync :: OnceLock :: new () ; if __ORIGINAL_TEMPLATE . get () . is_none () { _ = __ORIGINAL_TEMPLATE . set (dioxus_core :: internal :: HotReloadedTemplate :: new (None , :: alloc :: vec :: Vec :: new () , :: alloc :: vec :: Vec :: new () , :: alloc :: vec :: Vec :: new () , __TEMPLATE_ROOTS)) ; } __ORIGINAL_TEMPLATE . get () . unwrap () } # [cfg (debug_assertions)] let __template_read = { static __NORMALIZED_FILE : & 'static str = { const PATH : & str = :: const_format :: pmr :: __AssertStr { x : { const ARGS_OSRCTFL4A : :: const_format :: __str_methods :: ReplaceInput = :: const_format :: __str_methods :: ReplaceInputConv ("packages/playwright-tests/wasm-split-harness/src/main.rs" , "\\\\" , "/") . conv () ; { const OB : & [:: const_format :: pmr :: u8 ; ARGS_OSRCTFL4A . replace_length ()] = & ARGS_OSRCTFL4A . replace () ; const OS : & :: const_format :: pmr :: str = unsafe { { let bytes : & 'static [:: const_format :: pmr :: u8] = OB ; let string : & 'static :: const_format :: pmr :: str = { :: const_format :: __hidden_utils :: PtrToRef { ptr : bytes as * const [:: const_format :: pmr :: u8] as * const str , } . reff } ; string } } ; OS } } , } . x ; :: const_format :: pmr :: __AssertStr { x : { const ARGS_OSRCTFL4A : :: const_format :: __str_methods :: ReplaceInput = :: const_format :: __str_methods :: ReplaceInputConv (PATH , '\\' , "/") . conv () ; { const OB : & [:: const_format :: pmr :: u8 ; ARGS_OSRCTFL4A . replace_length ()] = & ARGS_OSRCTFL4A . replace () ; const OS : & :: const_format :: pmr :: str = unsafe { { let bytes : & 'static [:: const_format :: pmr :: u8] = OB ; let string : & 'static :: const_format :: pmr :: str = { :: const_format :: __hidden_utils :: PtrToRef { ptr : bytes as * const [:: const_format :: pmr :: u8] as * const str , } . reff } ; string } } ; OS } } , } . x } ; static __TEMPLATE : GlobalSignal < Option < dioxus_core :: internal :: HotReloadedTemplate > > = GlobalSignal :: with_location (| | None :: < dioxus_core :: internal :: HotReloadedTemplate > , __NORMALIZED_FILE , 27u32 , 5u32 , 2usize) ; dioxus_core :: Runtime :: current () . ok () . map (| _ | __TEMPLATE . read ()) } ; # [cfg (debug_assertions)] let __template_read = match __template_read . as_ref () . map (| __template_read | __template_read . as_ref ()) { Some (Some (__template_read)) => & __template_read , _ => __original_template () , } ; # [cfg (debug_assertions)] let mut __dynamic_literal_pool = dioxus_core :: internal :: DynamicLiteralPool :: new (:: alloc :: vec :: Vec :: new ()) ; let __dynamic_nodes : [dioxus_core :: DynamicNode ; 0usize] = [] ; let __dynamic_attributes : [Box < [dioxus_core :: Attribute] > ; 0usize] = [] ; # [doc (hidden)] static __TEMPLATE_ROOTS : & [dioxus_core :: TemplateNode] = & [dioxus_core :: TemplateNode :: Text { text : "About" }] ; # [cfg (debug_assertions)] { let mut __dynamic_value_pool = dioxus_core :: internal :: DynamicValuePool :: new (Vec :: from (__dynamic_nodes) , Vec :: from (__dynamic_attributes) , __dynamic_literal_pool) ; __dynamic_value_pool . render_with (__template_read) } }) }) . build () }) . into_vcomponent (Link) ;
                        __comp
                    }),
                    dioxus_core::DynamicNode::Component({
                        use dioxus_core::prelude::Properties;
                        let __comp = ({ fc_to_builder(Outlet::<Route>).build() })
                            .into_vcomponent(Outlet::<Route>);
                        __comp
                    }),
                ];
                let __dynamic_attributes: [Box<[dioxus_core::Attribute]>; 0usize] = [];
                #[doc(hidden)]
                static __TEMPLATE_ROOTS: &[dioxus_core::TemplateNode] = &[
                    {
                        dioxus_core::TemplateNode::Element {
                            tag: dioxus_elements::elements::nav::TAG_NAME,
                            namespace: dioxus_elements::nav::NAME_SPACE,
                            attrs: &[],
                            children: &[
                                dioxus_core::TemplateNode::Dynamic { id: 0usize },
                                dioxus_core::TemplateNode::Dynamic { id: 1usize },
                            ],
                        }
                    },
                    {
                        dioxus_core::TemplateNode::Element {
                            tag: dioxus_elements::elements::div::TAG_NAME,
                            namespace: dioxus_elements::div::NAME_SPACE,
                            attrs: &[],
                            children: &[dioxus_core::TemplateNode::Dynamic { id: 2usize }],
                        }
                    },
                ];
                #[cfg(debug_assertions)]
                {
                    let mut __dynamic_value_pool = dioxus_core::internal::DynamicValuePool::new(
                        Vec::from(__dynamic_nodes),
                        Vec::from(__dynamic_attributes),
                        __dynamic_literal_pool,
                    );
                    __dynamic_value_pool.render_with(__template_read)
                }
            })
        }
    }
}
#[allow(non_snake_case)]
#[doc(hidden)]
mod Nav_completions {
    #[doc(hidden)]
    #[allow(non_camel_case_types)]
    /// This enum is generated to help autocomplete the braces after the component. It does nothing
    pub enum Component {
        Nav {},
    }
}
#[allow(unused)]
use Nav_completions::Component::Nav;
#[allow(non_snake_case)]
fn Home() -> Element {
    {
        dioxus_core::internal::verify_component_called_as_component(Home);
        {
            dioxus_core::Element::Ok({
                #[cfg(debug_assertions)]
                fn __original_template() -> &'static dioxus_core::internal::HotReloadedTemplate {
                    static __ORIGINAL_TEMPLATE: ::std::sync::OnceLock<
                        dioxus_core::internal::HotReloadedTemplate,
                    > = ::std::sync::OnceLock::new();
                    if __ORIGINAL_TEMPLATE.get().is_none() {
                        _ = __ORIGINAL_TEMPLATE.set(
                            dioxus_core::internal::HotReloadedTemplate::new(
                                None,
                                ::alloc::vec::Vec::new(),
                                ::alloc::vec::Vec::new(),
                                ::alloc::vec::Vec::new(),
                                __TEMPLATE_ROOTS,
                            ),
                        );
                    }
                    __ORIGINAL_TEMPLATE.get().unwrap()
                }
                #[cfg(debug_assertions)]
                let __template_read = {
                    static __NORMALIZED_FILE: &'static str = {
                        const PATH: &str = ::const_format::pmr::__AssertStr {
                            x: {
                                const ARGS_OSRCTFL4A: ::const_format::__str_methods::ReplaceInput =
                                    ::const_format::__str_methods::ReplaceInputConv(
                                        "packages/playwright-tests/wasm-split-harness/src/main.rs",
                                        "\\\\",
                                        "/",
                                    )
                                    .conv();
                                {
                                    const OB: &[::const_format::pmr::u8;
                                         ARGS_OSRCTFL4A.replace_length()] =
                                        &ARGS_OSRCTFL4A.replace();
                                    const OS: &::const_format::pmr::str = unsafe {
                                        {
                                            let bytes: &'static [::const_format::pmr::u8] = OB;
                                            let string: &'static ::const_format::pmr::str = {
                                                ::const_format::__hidden_utils::PtrToRef {
                                                    ptr: bytes as *const [::const_format::pmr::u8]
                                                        as *const str,
                                                }
                                                .reff
                                            };
                                            string
                                        }
                                    };
                                    OS
                                }
                            },
                        }
                        .x;
                        ::const_format::pmr::__AssertStr {
                            x: {
                                const ARGS_OSRCTFL4A: ::const_format::__str_methods::ReplaceInput =
                                    ::const_format::__str_methods::ReplaceInputConv(PATH, '\\', "/")
                                        .conv();
                                {
                                    const OB: &[::const_format::pmr::u8;
                                         ARGS_OSRCTFL4A.replace_length()] =
                                        &ARGS_OSRCTFL4A.replace();
                                    const OS: &::const_format::pmr::str = unsafe {
                                        {
                                            let bytes: &'static [::const_format::pmr::u8] = OB;
                                            let string: &'static ::const_format::pmr::str = {
                                                ::const_format::__hidden_utils::PtrToRef {
                                                    ptr: bytes as *const [::const_format::pmr::u8]
                                                        as *const str,
                                                }
                                                .reff
                                            };
                                            string
                                        }
                                    };
                                    OS
                                }
                            },
                        }
                        .x
                    };
                    static __TEMPLATE: GlobalSignal<
                        Option<dioxus_core::internal::HotReloadedTemplate>,
                    > = GlobalSignal::with_location(
                        || None::<dioxus_core::internal::HotReloadedTemplate>,
                        __NORMALIZED_FILE,
                        40u32,
                        5u32,
                        0usize,
                    );
                    dioxus_core::Runtime::current()
                        .ok()
                        .map(|_| __TEMPLATE.read())
                };
                #[cfg(debug_assertions)]
                let __template_read = match __template_read
                    .as_ref()
                    .map(|__template_read| __template_read.as_ref())
                {
                    Some(Some(__template_read)) => &__template_read,
                    _ => __original_template(),
                };
                #[cfg(debug_assertions)]
                let mut __dynamic_literal_pool =
                    dioxus_core::internal::DynamicLiteralPool::new(::alloc::vec::Vec::new());
                let __dynamic_nodes: [dioxus_core::DynamicNode; 0usize] = [];
                let __dynamic_attributes: [Box<[dioxus_core::Attribute]>; 0usize] = [];
                #[doc(hidden)]
                static __TEMPLATE_ROOTS: &[dioxus_core::TemplateNode] = &[
                    {
                        dioxus_core::TemplateNode::Element {
                            tag: dioxus_elements::elements::h1::TAG_NAME,
                            namespace: dioxus_elements::h1::NAME_SPACE,
                            attrs: &[],
                            children: &[dioxus_core::TemplateNode::Text { text: "Home" }],
                        }
                    },
                    {
                        dioxus_core::TemplateNode::Element {
                            tag: dioxus_elements::elements::p::TAG_NAME,
                            namespace: dioxus_elements::p::NAME_SPACE,
                            attrs: &[],
                            children: &[dioxus_core::TemplateNode::Text {
                                text: "This is the home page",
                            }],
                        }
                    },
                ];
                #[cfg(debug_assertions)]
                {
                    let mut __dynamic_value_pool = dioxus_core::internal::DynamicValuePool::new(
                        Vec::from(__dynamic_nodes),
                        Vec::from(__dynamic_attributes),
                        __dynamic_literal_pool,
                    );
                    __dynamic_value_pool.render_with(__template_read)
                }
            })
        }
    }
}
#[allow(non_snake_case)]
#[doc(hidden)]
mod Home_completions {
    #[doc(hidden)]
    #[allow(non_camel_case_types)]
    /// This enum is generated to help autocomplete the braces after the component. It does nothing
    pub enum Component {
        Home {},
    }
}
#[allow(unused)]
use Home_completions::Component::Home;
#[allow(non_snake_case)]
fn About() -> Element {
    {
        dioxus_core::internal::verify_component_called_as_component(About);
        {
            dioxus_core::Element::Ok({
                #[cfg(debug_assertions)]
                fn __original_template() -> &'static dioxus_core::internal::HotReloadedTemplate {
                    static __ORIGINAL_TEMPLATE: ::std::sync::OnceLock<
                        dioxus_core::internal::HotReloadedTemplate,
                    > = ::std::sync::OnceLock::new();
                    if __ORIGINAL_TEMPLATE.get().is_none() {
                        _ = __ORIGINAL_TEMPLATE.set(
                            dioxus_core::internal::HotReloadedTemplate::new(
                                None,
                                ::alloc::vec::Vec::new(),
                                ::alloc::vec::Vec::new(),
                                ::alloc::vec::Vec::new(),
                                __TEMPLATE_ROOTS,
                            ),
                        );
                    }
                    __ORIGINAL_TEMPLATE.get().unwrap()
                }
                #[cfg(debug_assertions)]
                let __template_read = {
                    static __NORMALIZED_FILE: &'static str = {
                        const PATH: &str = ::const_format::pmr::__AssertStr {
                            x: {
                                const ARGS_OSRCTFL4A: ::const_format::__str_methods::ReplaceInput =
                                    ::const_format::__str_methods::ReplaceInputConv(
                                        "packages/playwright-tests/wasm-split-harness/src/main.rs",
                                        "\\\\",
                                        "/",
                                    )
                                    .conv();
                                {
                                    const OB: &[::const_format::pmr::u8;
                                         ARGS_OSRCTFL4A.replace_length()] =
                                        &ARGS_OSRCTFL4A.replace();
                                    const OS: &::const_format::pmr::str = unsafe {
                                        {
                                            let bytes: &'static [::const_format::pmr::u8] = OB;
                                            let string: &'static ::const_format::pmr::str = {
                                                ::const_format::__hidden_utils::PtrToRef {
                                                    ptr: bytes as *const [::const_format::pmr::u8]
                                                        as *const str,
                                                }
                                                .reff
                                            };
                                            string
                                        }
                                    };
                                    OS
                                }
                            },
                        }
                        .x;
                        ::const_format::pmr::__AssertStr {
                            x: {
                                const ARGS_OSRCTFL4A: ::const_format::__str_methods::ReplaceInput =
                                    ::const_format::__str_methods::ReplaceInputConv(PATH, '\\', "/")
                                        .conv();
                                {
                                    const OB: &[::const_format::pmr::u8;
                                         ARGS_OSRCTFL4A.replace_length()] =
                                        &ARGS_OSRCTFL4A.replace();
                                    const OS: &::const_format::pmr::str = unsafe {
                                        {
                                            let bytes: &'static [::const_format::pmr::u8] = OB;
                                            let string: &'static ::const_format::pmr::str = {
                                                ::const_format::__hidden_utils::PtrToRef {
                                                    ptr: bytes as *const [::const_format::pmr::u8]
                                                        as *const str,
                                                }
                                                .reff
                                            };
                                            string
                                        }
                                    };
                                    OS
                                }
                            },
                        }
                        .x
                    };
                    static __TEMPLATE: GlobalSignal<
                        Option<dioxus_core::internal::HotReloadedTemplate>,
                    > = GlobalSignal::with_location(
                        || None::<dioxus_core::internal::HotReloadedTemplate>,
                        __NORMALIZED_FILE,
                        48u32,
                        5u32,
                        0usize,
                    );
                    dioxus_core::Runtime::current()
                        .ok()
                        .map(|_| __TEMPLATE.read())
                };
                #[cfg(debug_assertions)]
                let __template_read = match __template_read
                    .as_ref()
                    .map(|__template_read| __template_read.as_ref())
                {
                    Some(Some(__template_read)) => &__template_read,
                    _ => __original_template(),
                };
                #[cfg(debug_assertions)]
                let mut __dynamic_literal_pool =
                    dioxus_core::internal::DynamicLiteralPool::new(::alloc::vec::Vec::new());
                let __dynamic_nodes: [dioxus_core::DynamicNode; 0usize] = [];
                let __dynamic_attributes: [Box<[dioxus_core::Attribute]>; 0usize] = [];
                #[doc(hidden)]
                static __TEMPLATE_ROOTS: &[dioxus_core::TemplateNode] = &[
                    {
                        dioxus_core::TemplateNode::Element {
                            tag: dioxus_elements::elements::h1::TAG_NAME,
                            namespace: dioxus_elements::h1::NAME_SPACE,
                            attrs: &[],
                            children: &[dioxus_core::TemplateNode::Text { text: "About" }],
                        }
                    },
                    {
                        dioxus_core::TemplateNode::Element {
                            tag: dioxus_elements::elements::p::TAG_NAME,
                            namespace: dioxus_elements::p::NAME_SPACE,
                            attrs: &[],
                            children: &[dioxus_core::TemplateNode::Text {
                                text: "This is the about page",
                            }],
                        }
                    },
                ];
                #[cfg(debug_assertions)]
                {
                    let mut __dynamic_value_pool = dioxus_core::internal::DynamicValuePool::new(
                        Vec::from(__dynamic_nodes),
                        Vec::from(__dynamic_attributes),
                        __dynamic_literal_pool,
                    );
                    __dynamic_value_pool.render_with(__template_read)
                }
            })
        }
    }
}
#[allow(non_snake_case)]
#[doc(hidden)]
mod About_completions {
    #[doc(hidden)]
    #[allow(non_camel_case_types)]
    /// This enum is generated to help autocomplete the braces after the component. It does nothing
    pub enum Component {
        About {},
    }
}
#[allow(unused)]
use About_completions::Component::About;
fn app() -> Element {
    let mut count = use_signal(|| 0);
    dioxus_core::Element::Ok({
        #[cfg(debug_assertions)]
        fn __original_template() -> &'static dioxus_core::internal::HotReloadedTemplate {
            static __ORIGINAL_TEMPLATE: ::std::sync::OnceLock<
                dioxus_core::internal::HotReloadedTemplate,
            > = ::std::sync::OnceLock::new();
            if __ORIGINAL_TEMPLATE.get().is_none() {
                _ = __ORIGINAL_TEMPLATE.set(dioxus_core::internal::HotReloadedTemplate::new(
                    None,
                    <[_]>::into_vec(::alloc::boxed::box_new([
                        dioxus_core::internal::HotReloadDynamicNode::Dynamic(0usize),
                    ])),
                    <[_]>::into_vec(::alloc::boxed::box_new([
                        dioxus_core::internal::HotReloadDynamicAttribute::Dynamic(0usize),
                        dioxus_core::internal::HotReloadDynamicAttribute::Dynamic(1usize),
                        dioxus_core::internal::HotReloadDynamicAttribute::Dynamic(2usize),
                        dioxus_core::internal::HotReloadDynamicAttribute::Dynamic(3usize),
                        dioxus_core::internal::HotReloadDynamicAttribute::Dynamic(4usize),
                    ])),
                    ::alloc::vec::Vec::new(),
                    __TEMPLATE_ROOTS,
                ));
            }
            __ORIGINAL_TEMPLATE.get().unwrap()
        }
        #[cfg(debug_assertions)]
        let __template_read = {
            static __NORMALIZED_FILE: &'static str = {
                const PATH: &str = ::const_format::pmr::__AssertStr {
                    x: {
                        const ARGS_OSRCTFL4A: ::const_format::__str_methods::ReplaceInput =
                            ::const_format::__str_methods::ReplaceInputConv(
                                "packages/playwright-tests/wasm-split-harness/src/main.rs",
                                "\\\\",
                                "/",
                            )
                            .conv();
                        {
                            const OB: &[::const_format::pmr::u8; ARGS_OSRCTFL4A.replace_length()] =
                                &ARGS_OSRCTFL4A.replace();
                            const OS: &::const_format::pmr::str = unsafe {
                                {
                                    let bytes: &'static [::const_format::pmr::u8] = OB;
                                    let string: &'static ::const_format::pmr::str = {
                                        ::const_format::__hidden_utils::PtrToRef {
                                            ptr: bytes as *const [::const_format::pmr::u8]
                                                as *const str,
                                        }
                                        .reff
                                    };
                                    string
                                }
                            };
                            OS
                        }
                    },
                }
                .x;
                ::const_format::pmr::__AssertStr {
                    x: {
                        const ARGS_OSRCTFL4A: ::const_format::__str_methods::ReplaceInput =
                            ::const_format::__str_methods::ReplaceInputConv(PATH, '\\', "/").conv();
                        {
                            const OB: &[::const_format::pmr::u8; ARGS_OSRCTFL4A.replace_length()] =
                                &ARGS_OSRCTFL4A.replace();
                            const OS: &::const_format::pmr::str = unsafe {
                                {
                                    let bytes: &'static [::const_format::pmr::u8] = OB;
                                    let string: &'static ::const_format::pmr::str = {
                                        ::const_format::__hidden_utils::PtrToRef {
                                            ptr: bytes as *const [::const_format::pmr::u8]
                                                as *const str,
                                        }
                                        .reff
                                    };
                                    string
                                }
                            };
                            OS
                        }
                    },
                }
                .x
            };
            static __TEMPLATE: GlobalSignal<Option<dioxus_core::internal::HotReloadedTemplate>> =
                GlobalSignal::with_location(
                    || None::<dioxus_core::internal::HotReloadedTemplate>,
                    __NORMALIZED_FILE,
                    57u32,
                    5u32,
                    0usize,
                );
            dioxus_core::Runtime::current()
                .ok()
                .map(|_| __TEMPLATE.read())
        };
        #[cfg(debug_assertions)]
        let __template_read = match __template_read
            .as_ref()
            .map(|__template_read| __template_read.as_ref())
        {
            Some(Some(__template_read)) => &__template_read,
            _ => __original_template(),
        };
        #[cfg(debug_assertions)]
        let mut __dynamic_literal_pool = dioxus_core::internal::DynamicLiteralPool::new(
            <[_]>::into_vec(::alloc::boxed::box_new([::alloc::__export::must_use({
                let res = ::alloc::fmt::format(format_args!("{0}", count));
                res
            })
            .to_string()])),
        );
        let __dynamic_nodes: [dioxus_core::DynamicNode; 1usize] = [dioxus_core::DynamicNode::Text(
            dioxus_core::VText::new(::alloc::__export::must_use({
                let res = ::alloc::fmt::format(format_args!("Count: {0}", count));
                res
            })),
        )];
        let __dynamic_attributes: [Box<[dioxus_core::Attribute]>; 5usize] = [
            Box::new([{
                dioxus_elements::events::onclick::call_with_explicit_closure(move |_| count += 1)
            }]),
            Box::new([{
                dioxus_elements::events::onclick::call_with_explicit_closure(move |_| {
                    add_body_text()
                })
            }]),
            Box::new([{
                dioxus_elements::events::onclick::call_with_explicit_closure(move |_| async move {
                    add_body_element().await;
                    count += 1;
                })
            }]),
            Box::new([{
                dioxus_elements::events::onclick::call_with_explicit_closure(move |_| gzip_it())
            }]),
            Box::new([{
                dioxus_elements::events::onclick::call_with_explicit_closure(move |_| brotli_it())
            }]),
        ];
        #[doc(hidden)]
        static __TEMPLATE_ROOTS: &[dioxus_core::TemplateNode] = &[
            {
                dioxus_core::TemplateNode::Element {
                    tag: dioxus_elements::elements::h1::TAG_NAME,
                    namespace: dioxus_elements::h1::NAME_SPACE,
                    attrs: &[],
                    children: &[dioxus_core::TemplateNode::Text {
                        text: "Hello bundle split",
                    }],
                }
            },
            {
                dioxus_core::TemplateNode::Element {
                    tag: dioxus_elements::elements::h3::TAG_NAME,
                    namespace: dioxus_elements::h3::NAME_SPACE,
                    attrs: &[],
                    children: &[dioxus_core::TemplateNode::Dynamic { id: 0usize }],
                }
            },
            {
                dioxus_core::TemplateNode::Element {
                    tag: dioxus_elements::elements::button::TAG_NAME,
                    namespace: dioxus_elements::button::NAME_SPACE,
                    attrs: &[dioxus_core::TemplateAttribute::Dynamic { id: 0usize }],
                    children: &[dioxus_core::TemplateNode::Text { text: "Click me" }],
                }
            },
            {
                dioxus_core::TemplateNode::Element {
                    tag: dioxus_elements::elements::button::TAG_NAME,
                    namespace: dioxus_elements::button::NAME_SPACE,
                    attrs: &[dioxus_core::TemplateAttribute::Dynamic { id: 1usize }],
                    children: &[dioxus_core::TemplateNode::Text {
                        text: "Add body text",
                    }],
                }
            },
            {
                dioxus_core::TemplateNode::Element {
                    tag: dioxus_elements::elements::button::TAG_NAME,
                    namespace: dioxus_elements::button::NAME_SPACE,
                    attrs: &[dioxus_core::TemplateAttribute::Dynamic { id: 2usize }],
                    children: &[dioxus_core::TemplateNode::Text {
                        text: "Add body element",
                    }],
                }
            },
            {
                dioxus_core::TemplateNode::Element {
                    tag: dioxus_elements::elements::button::TAG_NAME,
                    namespace: dioxus_elements::button::NAME_SPACE,
                    attrs: &[dioxus_core::TemplateAttribute::Dynamic { id: 3usize }],
                    children: &[dioxus_core::TemplateNode::Text { text: "GZIP it" }],
                }
            },
            {
                dioxus_core::TemplateNode::Element {
                    tag: dioxus_elements::elements::button::TAG_NAME,
                    namespace: dioxus_elements::button::NAME_SPACE,
                    attrs: &[dioxus_core::TemplateAttribute::Dynamic { id: 4usize }],
                    children: &[dioxus_core::TemplateNode::Text { text: "Brotli It" }],
                }
            },
            {
                dioxus_core::TemplateNode::Element {
                    tag: dioxus_elements::elements::div::TAG_NAME,
                    namespace: dioxus_elements::div::NAME_SPACE,
                    attrs: &[dioxus_core::TemplateAttribute::Static {
                        name: dioxus_elements::div::id.0,
                        namespace: dioxus_elements::div::id.1,
                        value: "output-box",
                    }],
                    children: &[],
                }
            },
        ];
        #[cfg(debug_assertions)]
        {
            let mut __dynamic_value_pool = dioxus_core::internal::DynamicValuePool::new(
                Vec::from(__dynamic_nodes),
                Vec::from(__dynamic_attributes),
                __dynamic_literal_pool,
            );
            __dynamic_value_pool.render_with(__template_read)
        }
    })
}
async fn add_body_text() {
    #[allow(improper_ctypes_definitions)]
    #[no_mangle]
    pub extern "C" fn __wasm_split_00one00_export_9e2ef41d9ee1132e8dd9ad244e172a1b_add_body_text() {
        let window = web_sys::window().unwrap_throw();
        let document = window.document().unwrap_throw();
        let output = document.create_text_node("Rendered!");
        let output_box = document.get_element_by_id("output-box").unwrap_throw();
        output_box.append_child(&output).unwrap_throw();
    }
    #[link(wasm_import_module = "./__wasm_split.js")]
    extern "C" {
        #[no_mangle]
        fn __wasm_split_load_one(
            callback: unsafe extern "C" fn(*const ::std::ffi::c_void, bool),
            data: *const ::std::ffi::c_void,
        ) -> ();
        #[allow(improper_ctypes)]
        #[no_mangle]
        fn __wasm_split_00one00_import_9e2ef41d9ee1132e8dd9ad244e172a1b_add_body_text();
    }
    const __wasm_split_loader: ::std::thread::LocalKey<::wasm_split::LazySplitLoader> = {
        #[inline]
        fn __init() -> ::wasm_split::LazySplitLoader {
            unsafe { ::wasm_split::LazySplitLoader::new(__wasm_split_load_one) }
        }
        unsafe {
            use ::std::thread::LocalKey;
            use ::std::thread::local_impl::LazyStorage;
            LocalKey::new(|init| {
                static VAL: LazyStorage<::wasm_split::LazySplitLoader> = LazyStorage::new();
                VAL.get(init, __init)
            })
        }
    };;
    if ::wasm_split::ensure_loaded(&__wasm_split_loader).await {
        unsafe { __wasm_split_00one00_import_9e2ef41d9ee1132e8dd9ad244e172a1b_add_body_text() }
    }
}
async fn add_body_element() {
    #[allow(improper_ctypes_definitions)]
    #[no_mangle]
    pub extern "C" fn __wasm_split_00two00_export_54a8f43a2e5ca90f8c3e68f09a621a97_add_body_element(
    ) {
        let window = web_sys::window().unwrap_throw();
        let document = window.document().unwrap_throw();
        let output = document.create_element("div").unwrap_throw();
        output.set_text_content(Some("Some inner div"));
        let output_box = document.get_element_by_id("output-box").unwrap_throw();
        output_box.append_child(&output).unwrap_throw();
        dioxus::prelude::queue_effect(move || {
            web_sys::console::log_1(&"add body async internal!".into());
        });
    }
    #[link(wasm_import_module = "./__wasm_split.js")]
    extern "C" {
        #[no_mangle]
        fn __wasm_split_load_two(
            callback: unsafe extern "C" fn(*const ::std::ffi::c_void, bool),
            data: *const ::std::ffi::c_void,
        ) -> ();
        #[allow(improper_ctypes)]
        #[no_mangle]
        fn __wasm_split_00two00_import_54a8f43a2e5ca90f8c3e68f09a621a97_add_body_element();
    }
    const __wasm_split_loader: ::std::thread::LocalKey<::wasm_split::LazySplitLoader> = {
        #[inline]
        fn __init() -> ::wasm_split::LazySplitLoader {
            unsafe { ::wasm_split::LazySplitLoader::new(__wasm_split_load_two) }
        }
        unsafe {
            use ::std::thread::LocalKey;
            use ::std::thread::local_impl::LazyStorage;
            LocalKey::new(|init| {
                static VAL: LazyStorage<::wasm_split::LazySplitLoader> = LazyStorage::new();
                VAL.get(init, __init)
            })
        }
    };;
    if ::wasm_split::ensure_loaded(&__wasm_split_loader).await {
        unsafe { __wasm_split_00two00_import_54a8f43a2e5ca90f8c3e68f09a621a97_add_body_element() }
    }
}
async fn brotli_it() {
    #[allow(improper_ctypes_definitions)]
    #[no_mangle]
    pub extern "C" fn __wasm_split_00three00_export_ebc1ec8ce99005b74a4a6cfacbf6723a_brotli_it() {
        static DATA: &[u8] = &[0u8; 10];
        let reader = Box::pin(futures::io::BufReader::new(DATA));
        let reader: Pin<Box<dyn futures::io::AsyncBufRead>> = reader;
        dioxus::prelude::spawn(async move {
            let mut fut = Box::pin(async_compression::futures::bufread::BrotliDecoder::new(
                reader,
            ));
            if fut.read_to_end(&mut Vec::new()).await.is_err() {
                web_sys::console::log_1(&"error reading brotli".into());
            }
        });
    }
    #[link(wasm_import_module = "./__wasm_split.js")]
    extern "C" {
        #[no_mangle]
        fn __wasm_split_load_three(
            callback: unsafe extern "C" fn(*const ::std::ffi::c_void, bool),
            data: *const ::std::ffi::c_void,
        ) -> ();
        #[allow(improper_ctypes)]
        #[no_mangle]
        fn __wasm_split_00three00_import_ebc1ec8ce99005b74a4a6cfacbf6723a_brotli_it();
    }
    const __wasm_split_loader: ::std::thread::LocalKey<::wasm_split::LazySplitLoader> = {
        #[inline]
        fn __init() -> ::wasm_split::LazySplitLoader {
            unsafe { ::wasm_split::LazySplitLoader::new(__wasm_split_load_three) }
        }
        unsafe {
            use ::std::thread::LocalKey;
            use ::std::thread::local_impl::LazyStorage;
            LocalKey::new(|init| {
                static VAL: LazyStorage<::wasm_split::LazySplitLoader> = LazyStorage::new();
                VAL.get(init, __init)
            })
        }
    };;
    if ::wasm_split::ensure_loaded(&__wasm_split_loader).await {
        unsafe { __wasm_split_00three00_import_ebc1ec8ce99005b74a4a6cfacbf6723a_brotli_it() }
    }
}
async fn gzip_it() {
    #[allow(improper_ctypes_definitions)]
    #[no_mangle]
    pub extern "C" fn __wasm_split_00four00_export_d42dec02c02d8c95a3c881650f5625f2_gzip_it() {
        static DATA: &[u8] = &[0u8; 10];
        let reader = Box::pin(futures::io::BufReader::new(DATA));
        let reader: Pin<Box<dyn futures::io::AsyncBufRead>> = reader;
        dioxus::prelude::spawn(async move {
            let mut fut = Box::pin(async_compression::futures::bufread::GzipDecoder::new(
                reader,
            ));
            if fut.read_to_end(&mut Vec::new()).await.is_err() {
                web_sys::console::log_1(&"error reading gzip".into());
            }
        });
    }
    #[link(wasm_import_module = "./__wasm_split.js")]
    extern "C" {
        #[no_mangle]
        fn __wasm_split_load_four(
            callback: unsafe extern "C" fn(*const ::std::ffi::c_void, bool),
            data: *const ::std::ffi::c_void,
        ) -> ();
        #[allow(improper_ctypes)]
        #[no_mangle]
        fn __wasm_split_00four00_import_d42dec02c02d8c95a3c881650f5625f2_gzip_it();
    }
    const __wasm_split_loader: ::std::thread::LocalKey<::wasm_split::LazySplitLoader> = {
        #[inline]
        fn __init() -> ::wasm_split::LazySplitLoader {
            unsafe { ::wasm_split::LazySplitLoader::new(__wasm_split_load_four) }
        }
        unsafe {
            use ::std::thread::LocalKey;
            use ::std::thread::local_impl::LazyStorage;
            LocalKey::new(|init| {
                static VAL: LazyStorage<::wasm_split::LazySplitLoader> = LazyStorage::new();
                VAL.get(init, __init)
            })
        }
    };;
    if ::wasm_split::ensure_loaded(&__wasm_split_loader).await {
        unsafe { __wasm_split_00four00_import_d42dec02c02d8c95a3c881650f5625f2_gzip_it() }
    }
}
