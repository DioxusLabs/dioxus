#![feature(prelude_import)]
//! Simple single-page-app setup.
//!
//!  Run with:
//!
//! ```sh
//! dx serve --platform web
//! ```
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use dioxus::{logger::tracing, prelude::*};
fn main() {
    dioxus::launch(app);
}
fn app() -> Element {
    let mut t = use_signal(|| 0);
    let mut text = use_signal(|| "...".to_string());
    dioxus_core::Element::Ok({
        fn __original_template() -> &'static dioxus_core::internal::HotReloadedTemplate {
            static __ORIGINAL_TEMPLATE: ::std::sync::OnceLock<
                dioxus_core::internal::HotReloadedTemplate,
            > = ::std::sync::OnceLock::new();
            if __ORIGINAL_TEMPLATE.get().is_none() {
                _ = __ORIGINAL_TEMPLATE
                    .set(
                        dioxus_core::internal::HotReloadedTemplate::new(
                            None,
                            <[_]>::into_vec(
                                ::alloc::boxed::box_new([
                                    dioxus_core::internal::HotReloadDynamicNode::Dynamic(
                                        0usize,
                                    ),
                                    dioxus_core::internal::HotReloadDynamicNode::Dynamic(
                                        1usize,
                                    ),
                                    dioxus_core::internal::HotReloadDynamicNode::Dynamic(
                                        2usize,
                                    ),
                                    dioxus_core::internal::HotReloadDynamicNode::Dynamic(
                                        3usize,
                                    ),
                                    dioxus_core::internal::HotReloadDynamicNode::Dynamic(
                                        4usize,
                                    ),
                                    dioxus_core::internal::HotReloadDynamicNode::Dynamic(
                                        5usize,
                                    ),
                                    dioxus_core::internal::HotReloadDynamicNode::Dynamic(
                                        6usize,
                                    ),
                                    dioxus_core::internal::HotReloadDynamicNode::Dynamic(
                                        7usize,
                                    ),
                                    dioxus_core::internal::HotReloadDynamicNode::Dynamic(
                                        8usize,
                                    ),
                                    dioxus_core::internal::HotReloadDynamicNode::Dynamic(
                                        9usize,
                                    ),
                                    dioxus_core::internal::HotReloadDynamicNode::Dynamic(
                                        10usize,
                                    ),
                                    dioxus_core::internal::HotReloadDynamicNode::Dynamic(
                                        11usize,
                                    ),
                                    dioxus_core::internal::HotReloadDynamicNode::Dynamic(
                                        12usize,
                                    ),
                                    dioxus_core::internal::HotReloadDynamicNode::Dynamic(
                                        13usize,
                                    ),
                                ]),
                            ),
                            <[_]>::into_vec(
                                ::alloc::boxed::box_new([
                                    dioxus_core::internal::HotReloadDynamicAttribute::Dynamic(
                                        0usize,
                                    ),
                                    dioxus_core::internal::HotReloadDynamicAttribute::Dynamic(
                                        1usize,
                                    ),
                                    dioxus_core::internal::HotReloadDynamicAttribute::Dynamic(
                                        2usize,
                                    ),
                                ]),
                            ),
                            <[_]>::into_vec(
                                ::alloc::boxed::box_new([
                                    dioxus_core::internal::HotReloadLiteral::Fmted(
                                        dioxus_core::internal::FmtedSegments::new(
                                            <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    dioxus_core::internal::FmtSegment::Literal {
                                                        value: "white",
                                                    },
                                                ]),
                                            ),
                                        ),
                                    ),
                                    dioxus_core::internal::HotReloadLiteral::Fmted(
                                        dioxus_core::internal::FmtedSegments::new(
                                            <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    dioxus_core::internal::FmtSegment::Literal {
                                                        value: "red",
                                                    },
                                                ]),
                                            ),
                                        ),
                                    ),
                                    dioxus_core::internal::HotReloadLiteral::Fmted(
                                        dioxus_core::internal::FmtedSegments::new(
                                            <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    dioxus_core::internal::FmtSegment::Literal {
                                                        value: "red",
                                                    },
                                                ]),
                                            ),
                                        ),
                                    ),
                                    dioxus_core::internal::HotReloadLiteral::Fmted(
                                        dioxus_core::internal::FmtedSegments::new(
                                            <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    dioxus_core::internal::FmtSegment::Literal {
                                                        value: "red",
                                                    },
                                                ]),
                                            ),
                                        ),
                                    ),
                                    dioxus_core::internal::HotReloadLiteral::Fmted(
                                        dioxus_core::internal::FmtedSegments::new(
                                            <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    dioxus_core::internal::FmtSegment::Literal {
                                                        value: "red",
                                                    },
                                                ]),
                                            ),
                                        ),
                                    ),
                                    dioxus_core::internal::HotReloadLiteral::Fmted(
                                        dioxus_core::internal::FmtedSegments::new(
                                            <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    dioxus_core::internal::FmtSegment::Literal {
                                                        value: "red",
                                                    },
                                                ]),
                                            ),
                                        ),
                                    ),
                                    dioxus_core::internal::HotReloadLiteral::Fmted(
                                        dioxus_core::internal::FmtedSegments::new(
                                            <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    dioxus_core::internal::FmtSegment::Literal {
                                                        value: "red",
                                                    },
                                                ]),
                                            ),
                                        ),
                                    ),
                                    dioxus_core::internal::HotReloadLiteral::Fmted(
                                        dioxus_core::internal::FmtedSegments::new(
                                            <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    dioxus_core::internal::FmtSegment::Literal {
                                                        value: "red",
                                                    },
                                                ]),
                                            ),
                                        ),
                                    ),
                                    dioxus_core::internal::HotReloadLiteral::Fmted(
                                        dioxus_core::internal::FmtedSegments::new(
                                            <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    dioxus_core::internal::FmtSegment::Literal {
                                                        value: "red",
                                                    },
                                                ]),
                                            ),
                                        ),
                                    ),
                                    dioxus_core::internal::HotReloadLiteral::Fmted(
                                        dioxus_core::internal::FmtedSegments::new(
                                            <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    dioxus_core::internal::FmtSegment::Literal {
                                                        value: "yellow",
                                                    },
                                                ]),
                                            ),
                                        ),
                                    ),
                                    dioxus_core::internal::HotReloadLiteral::Fmted(
                                        dioxus_core::internal::FmtedSegments::new(
                                            <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    dioxus_core::internal::FmtSegment::Literal {
                                                        value: "green",
                                                    },
                                                ]),
                                            ),
                                        ),
                                    ),
                                    dioxus_core::internal::HotReloadLiteral::Fmted(
                                        dioxus_core::internal::FmtedSegments::new(
                                            <[_]>::into_vec(
                                                ::alloc::boxed::box_new([
                                                    dioxus_core::internal::FmtSegment::Literal {
                                                        value: "red",
                                                    },
                                                ]),
                                            ),
                                        ),
                                    ),
                                ]),
                            ),
                            __TEMPLATE_ROOTS,
                        ),
                    );
            }
            __ORIGINAL_TEMPLATE.get().unwrap()
        }
        let __template_read = {
            static __NORMALIZED_FILE: &'static str = {
                const PATH: &str = ::const_format::pmr::__AssertStr {
                    x: {
                        const ARGS_OSRCTFL4A: ::const_format::__str_methods::ReplaceInput = ::const_format::__str_methods::ReplaceInputConv(
                                "examples/simple-spa-fullstack/src/main.rs",
                                "\\\\",
                                "/",
                            )
                            .conv();
                        {
                            const OB: &[::const_format::pmr::u8; ARGS_OSRCTFL4A
                                .replace_length()] = &ARGS_OSRCTFL4A.replace();
                            const OS: &::const_format::pmr::str = unsafe {
                                {
                                    let bytes: &'static [::const_format::pmr::u8] = OB;
                                    let string: &'static ::const_format::pmr::str = {
                                        ::const_format::__hidden_utils::PtrToRef {
                                            ptr: bytes as *const [::const_format::pmr::u8] as *const str,
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
                        const ARGS_OSRCTFL4A: ::const_format::__str_methods::ReplaceInput = ::const_format::__str_methods::ReplaceInputConv(
                                PATH,
                                '\\',
                                "/",
                            )
                            .conv();
                        {
                            const OB: &[::const_format::pmr::u8; ARGS_OSRCTFL4A
                                .replace_length()] = &ARGS_OSRCTFL4A.replace();
                            const OS: &::const_format::pmr::str = unsafe {
                                {
                                    let bytes: &'static [::const_format::pmr::u8] = OB;
                                    let string: &'static ::const_format::pmr::str = {
                                        ::const_format::__hidden_utils::PtrToRef {
                                            ptr: bytes as *const [::const_format::pmr::u8] as *const str,
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
                19u32,
                5u32,
                0usize,
            );
            dioxus_core::Runtime::current().ok().map(|_| __TEMPLATE.read())
        };
        let __template_read = match __template_read
            .as_ref()
            .map(|__template_read| __template_read.as_ref())
        {
            Some(Some(__template_read)) => &__template_read,
            _ => __original_template(),
        };
        let mut __dynamic_literal_pool = dioxus_core::internal::DynamicLiteralPool::new(
            <[_]>::into_vec(
                ::alloc::boxed::box_new([
                    ::alloc::__export::must_use({
                            let res = ::alloc::fmt::format(format_args!("{0}", t));
                            res
                        })
                        .to_string(),
                    ::alloc::__export::must_use({
                            let res = ::alloc::fmt::format(format_args!("{0}", text));
                            res
                        })
                        .to_string(),
                ]),
            ),
        );
        let __dynamic_nodes: [dioxus_core::DynamicNode; 14usize] = [
            dioxus_core::DynamicNode::Text(
                dioxus_core::VText::new(
                    ::alloc::__export::must_use({
                        let res = ::alloc::fmt::format(format_args!("Click me: {0}", t));
                        res
                    }),
                ),
            ),
            dioxus_core::DynamicNode::Component({
                use dioxus_core::prelude::Properties;
                let __comp = ({
                    fc_to_builder(EvalIt)
                        .color({
                            {
                                __dynamic_literal_pool
                                    .component_property(0usize, &*__template_read, "white")
                            }
                        })
                        .build()
                })
                    .into_vcomponent(EvalIt);
                __comp
            }),
            dioxus_core::DynamicNode::Component({
                use dioxus_core::prelude::Properties;
                let __comp = ({
                    fc_to_builder(EvalIt)
                        .color({
                            {
                                __dynamic_literal_pool
                                    .component_property(1usize, &*__template_read, "red")
                            }
                        })
                        .build()
                })
                    .into_vcomponent(EvalIt);
                __comp
            }),
            dioxus_core::DynamicNode::Component({
                use dioxus_core::prelude::Properties;
                let __comp = ({
                    fc_to_builder(EvalIt)
                        .color({
                            {
                                __dynamic_literal_pool
                                    .component_property(2usize, &*__template_read, "red")
                            }
                        })
                        .build()
                })
                    .into_vcomponent(EvalIt);
                __comp
            }),
            dioxus_core::DynamicNode::Component({
                use dioxus_core::prelude::Properties;
                let __comp = ({
                    fc_to_builder(EvalIt)
                        .color({
                            {
                                __dynamic_literal_pool
                                    .component_property(3usize, &*__template_read, "red")
                            }
                        })
                        .build()
                })
                    .into_vcomponent(EvalIt);
                __comp
            }),
            dioxus_core::DynamicNode::Component({
                use dioxus_core::prelude::Properties;
                let __comp = ({
                    fc_to_builder(EvalIt)
                        .color({
                            {
                                __dynamic_literal_pool
                                    .component_property(4usize, &*__template_read, "red")
                            }
                        })
                        .build()
                })
                    .into_vcomponent(EvalIt);
                __comp
            }),
            dioxus_core::DynamicNode::Component({
                use dioxus_core::prelude::Properties;
                let __comp = ({
                    fc_to_builder(EvalIt)
                        .color({
                            {
                                __dynamic_literal_pool
                                    .component_property(5usize, &*__template_read, "red")
                            }
                        })
                        .build()
                })
                    .into_vcomponent(EvalIt);
                __comp
            }),
            dioxus_core::DynamicNode::Component({
                use dioxus_core::prelude::Properties;
                let __comp = ({
                    fc_to_builder(EvalIt)
                        .color({
                            {
                                __dynamic_literal_pool
                                    .component_property(6usize, &*__template_read, "red")
                            }
                        })
                        .build()
                })
                    .into_vcomponent(EvalIt);
                __comp
            }),
            dioxus_core::DynamicNode::Component({
                use dioxus_core::prelude::Properties;
                let __comp = ({
                    fc_to_builder(EvalIt)
                        .color({
                            {
                                __dynamic_literal_pool
                                    .component_property(7usize, &*__template_read, "red")
                            }
                        })
                        .build()
                })
                    .into_vcomponent(EvalIt);
                __comp
            }),
            dioxus_core::DynamicNode::Component({
                use dioxus_core::prelude::Properties;
                let __comp = ({
                    fc_to_builder(EvalIt)
                        .color({
                            {
                                __dynamic_literal_pool
                                    .component_property(8usize, &*__template_read, "red")
                            }
                        })
                        .build()
                })
                    .into_vcomponent(EvalIt);
                __comp
            }),
            dioxus_core::DynamicNode::Component({
                use dioxus_core::prelude::Properties;
                let __comp = ({
                    fc_to_builder(EvalIt)
                        .color({
                            {
                                __dynamic_literal_pool
                                    .component_property(9usize, &*__template_read, "yellow")
                            }
                        })
                        .build()
                })
                    .into_vcomponent(EvalIt);
                __comp
            }),
            dioxus_core::DynamicNode::Component({
                use dioxus_core::prelude::Properties;
                let __comp = ({
                    fc_to_builder(EvalIt)
                        .color({
                            {
                                __dynamic_literal_pool
                                    .component_property(10usize, &*__template_read, "green")
                            }
                        })
                        .build()
                })
                    .into_vcomponent(EvalIt);
                __comp
            }),
            dioxus_core::DynamicNode::Component({
                use dioxus_core::prelude::Properties;
                let __comp = ({
                    fc_to_builder(EvalIt)
                        .color({
                            {
                                __dynamic_literal_pool
                                    .component_property(11usize, &*__template_read, "red")
                            }
                        })
                        .build()
                })
                    .into_vcomponent(EvalIt);
                __comp
            }),
            dioxus_core::DynamicNode::Text(
                dioxus_core::VText::new(
                    ::alloc::__export::must_use({
                        let res = ::alloc::fmt::format(
                            format_args!("Server said: {0}", text),
                        );
                        res
                    }),
                ),
            ),
        ];
        let __dynamic_attributes: [Box<[dioxus_core::Attribute]>; 3usize] = [
            Box::new([
                {
                    dioxus_elements::events::onclick::call_with_explicit_closure(move |
                        _|
                    t += 1)
                },
            ]),
            Box::new([
                {
                    dioxus_elements::events::onclick::call_with_explicit_closure(move |
                        _|
                    async move {
                        if let Ok(data) = get_server_data().await {
                            text.set(data.clone());
                            let res = post_server_data(data).await;
                            {
                                use ::tracing::__macro_support::Callsite as _;
                                static __CALLSITE: ::tracing::callsite::DefaultCallsite = {
                                    static META: ::tracing::Metadata<'static> = {
                                        ::tracing_core::metadata::Metadata::new(
                                            "event examples/simple-spa-fullstack/src/main.rs:41",
                                            "simple_spa_example_fullstack",
                                            ::tracing::Level::DEBUG,
                                            ::tracing_core::__macro_support::Option::Some(
                                                "examples/simple-spa-fullstack/src/main.rs",
                                            ),
                                            ::tracing_core::__macro_support::Option::Some(41u32),
                                            ::tracing_core::__macro_support::Option::Some(
                                                "simple_spa_example_fullstack",
                                            ),
                                            ::tracing_core::field::FieldSet::new(
                                                &["message"],
                                                ::tracing_core::callsite::Identifier(&__CALLSITE),
                                            ),
                                            ::tracing::metadata::Kind::EVENT,
                                        )
                                    };
                                    ::tracing::callsite::DefaultCallsite::new(&META)
                                };
                                let enabled = ::tracing::Level::DEBUG
                                    <= ::tracing::level_filters::STATIC_MAX_LEVEL
                                    && ::tracing::Level::DEBUG
                                        <= ::tracing::level_filters::LevelFilter::current()
                                    && {
                                        let interest = __CALLSITE.interest();
                                        !interest.is_never()
                                            && ::tracing::__macro_support::__is_enabled(
                                                __CALLSITE.metadata(),
                                                interest,
                                            )
                                    };
                                if enabled {
                                    (|value_set: ::tracing::field::ValueSet| {
                                        let meta = __CALLSITE.metadata();
                                        ::tracing::Event::dispatch(meta, &value_set);
                                    })({
                                        #[allow(unused_imports)]
                                        use ::tracing::field::{debug, display, Value};
                                        let mut iter = __CALLSITE.metadata().fields().iter();
                                        __CALLSITE
                                            .metadata()
                                            .fields()
                                            .value_set(
                                                &[
                                                    (
                                                        &::tracing::__macro_support::Iterator::next(&mut iter)
                                                            .expect("FieldSet corrupted (this is a bug)"),
                                                        ::tracing::__macro_support::Option::Some(
                                                            &format_args!("res: {0:?}", res) as &dyn Value,
                                                        ),
                                                    ),
                                                ],
                                            )
                                    });
                                } else {
                                }
                            };
                        }
                    })
                },
            ]),
            Box::new([
                {
                    dioxus_elements::events::onclick::call_with_explicit_closure(move |
                        _|
                    async move {
                        if let Ok(data) = get_curr_time().await {
                            text.set(data.clone());
                            let res = post_server_data(data).await;
                            {
                                use ::tracing::__macro_support::Callsite as _;
                                static __CALLSITE: ::tracing::callsite::DefaultCallsite = {
                                    static META: ::tracing::Metadata<'static> = {
                                        ::tracing_core::metadata::Metadata::new(
                                            "event examples/simple-spa-fullstack/src/main.rs:52",
                                            "simple_spa_example_fullstack",
                                            ::tracing::Level::DEBUG,
                                            ::tracing_core::__macro_support::Option::Some(
                                                "examples/simple-spa-fullstack/src/main.rs",
                                            ),
                                            ::tracing_core::__macro_support::Option::Some(52u32),
                                            ::tracing_core::__macro_support::Option::Some(
                                                "simple_spa_example_fullstack",
                                            ),
                                            ::tracing_core::field::FieldSet::new(
                                                &["message"],
                                                ::tracing_core::callsite::Identifier(&__CALLSITE),
                                            ),
                                            ::tracing::metadata::Kind::EVENT,
                                        )
                                    };
                                    ::tracing::callsite::DefaultCallsite::new(&META)
                                };
                                let enabled = ::tracing::Level::DEBUG
                                    <= ::tracing::level_filters::STATIC_MAX_LEVEL
                                    && ::tracing::Level::DEBUG
                                        <= ::tracing::level_filters::LevelFilter::current()
                                    && {
                                        let interest = __CALLSITE.interest();
                                        !interest.is_never()
                                            && ::tracing::__macro_support::__is_enabled(
                                                __CALLSITE.metadata(),
                                                interest,
                                            )
                                    };
                                if enabled {
                                    (|value_set: ::tracing::field::ValueSet| {
                                        let meta = __CALLSITE.metadata();
                                        ::tracing::Event::dispatch(meta, &value_set);
                                    })({
                                        #[allow(unused_imports)]
                                        use ::tracing::field::{debug, display, Value};
                                        let mut iter = __CALLSITE.metadata().fields().iter();
                                        __CALLSITE
                                            .metadata()
                                            .fields()
                                            .value_set(
                                                &[
                                                    (
                                                        &::tracing::__macro_support::Iterator::next(&mut iter)
                                                            .expect("FieldSet corrupted (this is a bug)"),
                                                        ::tracing::__macro_support::Option::Some(
                                                            &format_args!("res: {0:?}", res) as &dyn Value,
                                                        ),
                                                    ),
                                                ],
                                            )
                                    });
                                } else {
                                }
                            };
                        }
                    })
                },
            ]),
        ];
        #[doc(hidden)]
        static __TEMPLATE_ROOTS: &[dioxus_core::TemplateNode] = &[
            {
                dioxus_core::TemplateNode::Element {
                    tag: dioxus_elements::elements::h1::TAG_NAME,
                    namespace: dioxus_elements::h1::NAME_SPACE,
                    attrs: &[],
                    children: &[
                        dioxus_core::TemplateNode::Text {
                            text: "Set your favorite color",
                        },
                    ],
                }
            },
            {
                dioxus_core::TemplateNode::Element {
                    tag: dioxus_elements::elements::button::TAG_NAME,
                    namespace: dioxus_elements::button::NAME_SPACE,
                    attrs: &[
                        dioxus_core::TemplateAttribute::Dynamic {
                            id: 0usize,
                        },
                    ],
                    children: &[
                        dioxus_core::TemplateNode::Dynamic {
                            id: 0usize,
                        },
                    ],
                }
            },
            {
                dioxus_core::TemplateNode::Element {
                    tag: dioxus_elements::elements::div::TAG_NAME,
                    namespace: dioxus_elements::div::NAME_SPACE,
                    attrs: &[],
                    children: &[
                        dioxus_core::TemplateNode::Dynamic {
                            id: 1usize,
                        },
                        dioxus_core::TemplateNode::Dynamic {
                            id: 2usize,
                        },
                        dioxus_core::TemplateNode::Dynamic {
                            id: 3usize,
                        },
                        dioxus_core::TemplateNode::Dynamic {
                            id: 4usize,
                        },
                        dioxus_core::TemplateNode::Dynamic {
                            id: 5usize,
                        },
                        dioxus_core::TemplateNode::Dynamic {
                            id: 6usize,
                        },
                        dioxus_core::TemplateNode::Dynamic {
                            id: 7usize,
                        },
                        dioxus_core::TemplateNode::Dynamic {
                            id: 8usize,
                        },
                        dioxus_core::TemplateNode::Dynamic {
                            id: 9usize,
                        },
                        dioxus_core::TemplateNode::Dynamic {
                            id: 10usize,
                        },
                        dioxus_core::TemplateNode::Dynamic {
                            id: 11usize,
                        },
                        dioxus_core::TemplateNode::Dynamic {
                            id: 12usize,
                        },
                    ],
                }
            },
            {
                dioxus_core::TemplateNode::Element {
                    tag: dioxus_elements::elements::button::TAG_NAME,
                    namespace: dioxus_elements::button::NAME_SPACE,
                    attrs: &[
                        dioxus_core::TemplateAttribute::Dynamic {
                            id: 1usize,
                        },
                    ],
                    children: &[
                        dioxus_core::TemplateNode::Text {
                            text: "Run a server function!",
                        },
                    ],
                }
            },
            {
                dioxus_core::TemplateNode::Element {
                    tag: dioxus_elements::elements::button::TAG_NAME,
                    namespace: dioxus_elements::button::NAME_SPACE,
                    attrs: &[
                        dioxus_core::TemplateAttribute::Dynamic {
                            id: 2usize,
                        },
                    ],
                    children: &[
                        dioxus_core::TemplateNode::Text {
                            text: "Run a server function with data!",
                        },
                    ],
                }
            },
            dioxus_core::TemplateNode::Dynamic {
                id: 13usize,
            },
        ];
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
/**Serialized arguments for the [`post_server_data`] server function.

*/
#[serde(crate = "server_fn::serde")]
struct PostServerData {
    data: String,
}
#[automatically_derived]
impl ::core::fmt::Debug for PostServerData {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::debug_struct_field1_finish(
            f,
            "PostServerData",
            "data",
            &&self.data,
        )
    }
}
#[automatically_derived]
impl ::core::clone::Clone for PostServerData {
    #[inline]
    fn clone(&self) -> PostServerData {
        PostServerData {
            data: ::core::clone::Clone::clone(&self.data),
        }
    }
}
#[doc(hidden)]
#[allow(
    non_upper_case_globals,
    unused_attributes,
    unused_qualifications,
    clippy::absolute_paths,
)]
const _: () = {
    use server_fn::serde as _serde;
    #[automatically_derived]
    impl server_fn::serde::Serialize for PostServerData {
        fn serialize<__S>(
            &self,
            __serializer: __S,
        ) -> server_fn::serde::__private::Result<__S::Ok, __S::Error>
        where
            __S: server_fn::serde::Serializer,
        {
            let mut __serde_state = _serde::Serializer::serialize_struct(
                __serializer,
                "PostServerData",
                false as usize + 1,
            )?;
            _serde::ser::SerializeStruct::serialize_field(
                &mut __serde_state,
                "data",
                &self.data,
            )?;
            _serde::ser::SerializeStruct::end(__serde_state)
        }
    }
};
#[doc(hidden)]
#[allow(
    non_upper_case_globals,
    unused_attributes,
    unused_qualifications,
    clippy::absolute_paths,
)]
const _: () = {
    use server_fn::serde as _serde;
    #[automatically_derived]
    impl<'de> server_fn::serde::Deserialize<'de> for PostServerData {
        fn deserialize<__D>(
            __deserializer: __D,
        ) -> server_fn::serde::__private::Result<Self, __D::Error>
        where
            __D: server_fn::serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            enum __Field {
                __field0,
                __ignore,
            }
            #[doc(hidden)]
            struct __FieldVisitor;
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(
                        __formatter,
                        "field identifier",
                    )
                }
                fn visit_u64<__E>(
                    self,
                    __value: u64,
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        0u64 => _serde::__private::Ok(__Field::__field0),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
                fn visit_str<__E>(
                    self,
                    __value: &str,
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        "data" => _serde::__private::Ok(__Field::__field0),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        b"data" => _serde::__private::Ok(__Field::__field0),
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
            }
            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(
                        __deserializer,
                        __FieldVisitor,
                    )
                }
            }
            #[doc(hidden)]
            struct __Visitor<'de> {
                marker: _serde::__private::PhantomData<PostServerData>,
                lifetime: _serde::__private::PhantomData<&'de ()>,
            }
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                type Value = PostServerData;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(
                        __formatter,
                        "struct PostServerData",
                    )
                }
                #[inline]
                fn visit_seq<__A>(
                    self,
                    mut __seq: __A,
                ) -> _serde::__private::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::SeqAccess<'de>,
                {
                    let __field0 = match _serde::de::SeqAccess::next_element::<
                        String,
                    >(&mut __seq)? {
                        _serde::__private::Some(__value) => __value,
                        _serde::__private::None => {
                            return _serde::__private::Err(
                                _serde::de::Error::invalid_length(
                                    0usize,
                                    &"struct PostServerData with 1 element",
                                ),
                            );
                        }
                    };
                    _serde::__private::Ok(PostServerData { data: __field0 })
                }
                #[inline]
                fn visit_map<__A>(
                    self,
                    mut __map: __A,
                ) -> _serde::__private::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::MapAccess<'de>,
                {
                    let mut __field0: _serde::__private::Option<String> = _serde::__private::None;
                    while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                        __Field,
                    >(&mut __map)? {
                        match __key {
                            __Field::__field0 => {
                                if _serde::__private::Option::is_some(&__field0) {
                                    return _serde::__private::Err(
                                        <__A::Error as _serde::de::Error>::duplicate_field("data"),
                                    );
                                }
                                __field0 = _serde::__private::Some(
                                    _serde::de::MapAccess::next_value::<String>(&mut __map)?,
                                );
                            }
                            _ => {
                                let _ = _serde::de::MapAccess::next_value::<
                                    _serde::de::IgnoredAny,
                                >(&mut __map)?;
                            }
                        }
                    }
                    let __field0 = match __field0 {
                        _serde::__private::Some(__field0) => __field0,
                        _serde::__private::None => {
                            _serde::__private::de::missing_field("data")?
                        }
                    };
                    _serde::__private::Ok(PostServerData { data: __field0 })
                }
            }
            #[doc(hidden)]
            const FIELDS: &'static [&'static str] = &["data"];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "PostServerData",
                FIELDS,
                __Visitor {
                    marker: _serde::__private::PhantomData::<PostServerData>,
                    lifetime: _serde::__private::PhantomData,
                },
            )
        }
    }
};
impl From<PostServerData> for String {
    fn from(value: PostServerData) -> Self {
        let PostServerData { data } = value;
        data
    }
}
impl From<String> for PostServerData {
    fn from(data: String) -> Self {
        PostServerData { data }
    }
}
impl std::ops::Deref for PostServerData {
    type Target = String;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}
impl server_fn::ServerFn for PostServerData {
    const PATH: &'static str = ::const_format::pmr::__AssertStr {
        x: {
            use ::const_format::__cf_osRcTFl4A;
            ({
                #[doc(hidden)]
                #[allow(unused_mut, non_snake_case)]
                const CONCATP_NHPMWYD3NJA: &[__cf_osRcTFl4A::pmr::PArgument] = {
                    let fmt = __cf_osRcTFl4A::pmr::FormattingFlags::NEW;
                    &[
                        __cf_osRcTFl4A::pmr::PConvWrapper("/api")
                            .to_pargument_display(fmt),
                        __cf_osRcTFl4A::pmr::PConvWrapper("").to_pargument_display(fmt),
                        __cf_osRcTFl4A::pmr::PConvWrapper("/post_server_data")
                            .to_pargument_display(fmt),
                    ]
                };
                {
                    #[doc(hidden)]
                    const ARR_LEN: usize = ::const_format::pmr::PArgument::calc_len(
                        CONCATP_NHPMWYD3NJA,
                    );
                    #[doc(hidden)]
                    const CONCAT_ARR: &::const_format::pmr::LenAndArray<[u8; ARR_LEN]> = &::const_format::pmr::__priv_concatenate(
                        CONCATP_NHPMWYD3NJA,
                    );
                    #[doc(hidden)]
                    #[allow(clippy::transmute_ptr_to_ptr)]
                    const CONCAT_STR: &str = unsafe {
                        let slice = ::const_format::pmr::transmute::<
                            &[u8; ARR_LEN],
                            &[u8; CONCAT_ARR.len],
                        >(&CONCAT_ARR.array);
                        {
                            let bytes: &'static [::const_format::pmr::u8] = slice;
                            let string: &'static ::const_format::pmr::str = {
                                ::const_format::__hidden_utils::PtrToRef {
                                    ptr: bytes as *const [::const_format::pmr::u8] as *const str,
                                }
                                    .reff
                            };
                            string
                        }
                    };
                    CONCAT_STR
                }
            })
        },
    }
        .x;
    type Client = server_fn::client::browser::BrowserClient;
    type Server = server_fn::mock::BrowserMockServer;
    type Protocol = server_fn::Http<server_fn::codec::PostUrl, server_fn::codec::Json>;
    type Output = ();
    type Error = ServerFnError;
    type InputStreamError = ServerFnError;
    type OutputStreamError = ServerFnError;
    fn middlewares() -> Vec<
        std::sync::Arc<
            dyn server_fn::middleware::Layer<
                <Self::Server as server_fn::server::Server<Self::Error>>::Request,
                <Self::Server as server_fn::server::Server<Self::Error>>::Response,
            >,
        >,
    > {
        ::alloc::vec::Vec::new()
    }
    #[allow(unused_variables)]
    async fn run_body(self) -> Result<(), ServerFnError> {
        ::core::panicking::panic("internal error: entered unreachable code")
    }
}
#[allow(unused_variables)]
async fn post_server_data(data: String) -> Result<(), ServerFnError> {
    use server_fn::ServerFn;
    let data = PostServerData { data };
    data.run_on_client().await
}
/**Serialized arguments for the [`get_server_data`] server function.

*/
#[serde(crate = "server_fn::serde")]
struct GetServerData {}
#[automatically_derived]
impl ::core::fmt::Debug for GetServerData {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(f, "GetServerData")
    }
}
#[automatically_derived]
impl ::core::clone::Clone for GetServerData {
    #[inline]
    fn clone(&self) -> GetServerData {
        GetServerData {}
    }
}
#[doc(hidden)]
#[allow(
    non_upper_case_globals,
    unused_attributes,
    unused_qualifications,
    clippy::absolute_paths,
)]
const _: () = {
    use server_fn::serde as _serde;
    #[automatically_derived]
    impl server_fn::serde::Serialize for GetServerData {
        fn serialize<__S>(
            &self,
            __serializer: __S,
        ) -> server_fn::serde::__private::Result<__S::Ok, __S::Error>
        where
            __S: server_fn::serde::Serializer,
        {
            let __serde_state = _serde::Serializer::serialize_struct(
                __serializer,
                "GetServerData",
                false as usize,
            )?;
            _serde::ser::SerializeStruct::end(__serde_state)
        }
    }
};
#[doc(hidden)]
#[allow(
    non_upper_case_globals,
    unused_attributes,
    unused_qualifications,
    clippy::absolute_paths,
)]
const _: () = {
    use server_fn::serde as _serde;
    #[automatically_derived]
    impl<'de> server_fn::serde::Deserialize<'de> for GetServerData {
        fn deserialize<__D>(
            __deserializer: __D,
        ) -> server_fn::serde::__private::Result<Self, __D::Error>
        where
            __D: server_fn::serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            enum __Field {
                __ignore,
            }
            #[doc(hidden)]
            struct __FieldVisitor;
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(
                        __formatter,
                        "field identifier",
                    )
                }
                fn visit_u64<__E>(
                    self,
                    __value: u64,
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
                fn visit_str<__E>(
                    self,
                    __value: &str,
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
            }
            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(
                        __deserializer,
                        __FieldVisitor,
                    )
                }
            }
            #[doc(hidden)]
            struct __Visitor<'de> {
                marker: _serde::__private::PhantomData<GetServerData>,
                lifetime: _serde::__private::PhantomData<&'de ()>,
            }
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                type Value = GetServerData;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(
                        __formatter,
                        "struct GetServerData",
                    )
                }
                #[inline]
                fn visit_seq<__A>(
                    self,
                    _: __A,
                ) -> _serde::__private::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::SeqAccess<'de>,
                {
                    _serde::__private::Ok(GetServerData {})
                }
                #[inline]
                fn visit_map<__A>(
                    self,
                    mut __map: __A,
                ) -> _serde::__private::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::MapAccess<'de>,
                {
                    while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                        __Field,
                    >(&mut __map)? {
                        match __key {
                            _ => {
                                let _ = _serde::de::MapAccess::next_value::<
                                    _serde::de::IgnoredAny,
                                >(&mut __map)?;
                            }
                        }
                    }
                    _serde::__private::Ok(GetServerData {})
                }
            }
            #[doc(hidden)]
            const FIELDS: &'static [&'static str] = &[];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "GetServerData",
                FIELDS,
                __Visitor {
                    marker: _serde::__private::PhantomData::<GetServerData>,
                    lifetime: _serde::__private::PhantomData,
                },
            )
        }
    }
};
impl server_fn::ServerFn for GetServerData {
    const PATH: &'static str = ::const_format::pmr::__AssertStr {
        x: {
            use ::const_format::__cf_osRcTFl4A;
            ({
                #[doc(hidden)]
                #[allow(unused_mut, non_snake_case)]
                const CONCATP_NHPMWYD3NJA: &[__cf_osRcTFl4A::pmr::PArgument] = {
                    let fmt = __cf_osRcTFl4A::pmr::FormattingFlags::NEW;
                    &[
                        __cf_osRcTFl4A::pmr::PConvWrapper("/api")
                            .to_pargument_display(fmt),
                        __cf_osRcTFl4A::pmr::PConvWrapper("").to_pargument_display(fmt),
                        __cf_osRcTFl4A::pmr::PConvWrapper("/get_server_data")
                            .to_pargument_display(fmt),
                    ]
                };
                {
                    #[doc(hidden)]
                    const ARR_LEN: usize = ::const_format::pmr::PArgument::calc_len(
                        CONCATP_NHPMWYD3NJA,
                    );
                    #[doc(hidden)]
                    const CONCAT_ARR: &::const_format::pmr::LenAndArray<[u8; ARR_LEN]> = &::const_format::pmr::__priv_concatenate(
                        CONCATP_NHPMWYD3NJA,
                    );
                    #[doc(hidden)]
                    #[allow(clippy::transmute_ptr_to_ptr)]
                    const CONCAT_STR: &str = unsafe {
                        let slice = ::const_format::pmr::transmute::<
                            &[u8; ARR_LEN],
                            &[u8; CONCAT_ARR.len],
                        >(&CONCAT_ARR.array);
                        {
                            let bytes: &'static [::const_format::pmr::u8] = slice;
                            let string: &'static ::const_format::pmr::str = {
                                ::const_format::__hidden_utils::PtrToRef {
                                    ptr: bytes as *const [::const_format::pmr::u8] as *const str,
                                }
                                    .reff
                            };
                            string
                        }
                    };
                    CONCAT_STR
                }
            })
        },
    }
        .x;
    type Client = server_fn::client::browser::BrowserClient;
    type Server = server_fn::mock::BrowserMockServer;
    type Protocol = server_fn::Http<server_fn::codec::PostUrl, server_fn::codec::Json>;
    type Output = String;
    type Error = ServerFnError;
    type InputStreamError = ServerFnError;
    type OutputStreamError = ServerFnError;
    fn middlewares() -> Vec<
        std::sync::Arc<
            dyn server_fn::middleware::Layer<
                <Self::Server as server_fn::server::Server<Self::Error>>::Request,
                <Self::Server as server_fn::server::Server<Self::Error>>::Response,
            >,
        >,
    > {
        ::alloc::vec::Vec::new()
    }
    #[allow(unused_variables)]
    async fn run_body(self) -> Result<String, ServerFnError> {
        ::core::panicking::panic("internal error: entered unreachable code")
    }
}
#[allow(unused_variables)]
async fn get_server_data() -> Result<String, ServerFnError> {
    use server_fn::ServerFn;
    let data = GetServerData {};
    data.run_on_client().await
}
/**Serialized arguments for the [`get_curr_time`] server function.

*/
#[serde(crate = "server_fn::serde")]
struct GetCurrTime {}
#[automatically_derived]
impl ::core::fmt::Debug for GetCurrTime {
    #[inline]
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        ::core::fmt::Formatter::write_str(f, "GetCurrTime")
    }
}
#[automatically_derived]
impl ::core::clone::Clone for GetCurrTime {
    #[inline]
    fn clone(&self) -> GetCurrTime {
        GetCurrTime {}
    }
}
#[doc(hidden)]
#[allow(
    non_upper_case_globals,
    unused_attributes,
    unused_qualifications,
    clippy::absolute_paths,
)]
const _: () = {
    use server_fn::serde as _serde;
    #[automatically_derived]
    impl server_fn::serde::Serialize for GetCurrTime {
        fn serialize<__S>(
            &self,
            __serializer: __S,
        ) -> server_fn::serde::__private::Result<__S::Ok, __S::Error>
        where
            __S: server_fn::serde::Serializer,
        {
            let __serde_state = _serde::Serializer::serialize_struct(
                __serializer,
                "GetCurrTime",
                false as usize,
            )?;
            _serde::ser::SerializeStruct::end(__serde_state)
        }
    }
};
#[doc(hidden)]
#[allow(
    non_upper_case_globals,
    unused_attributes,
    unused_qualifications,
    clippy::absolute_paths,
)]
const _: () = {
    use server_fn::serde as _serde;
    #[automatically_derived]
    impl<'de> server_fn::serde::Deserialize<'de> for GetCurrTime {
        fn deserialize<__D>(
            __deserializer: __D,
        ) -> server_fn::serde::__private::Result<Self, __D::Error>
        where
            __D: server_fn::serde::Deserializer<'de>,
        {
            #[allow(non_camel_case_types)]
            #[doc(hidden)]
            enum __Field {
                __ignore,
            }
            #[doc(hidden)]
            struct __FieldVisitor;
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __FieldVisitor {
                type Value = __Field;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(
                        __formatter,
                        "field identifier",
                    )
                }
                fn visit_u64<__E>(
                    self,
                    __value: u64,
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
                fn visit_str<__E>(
                    self,
                    __value: &str,
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
                fn visit_bytes<__E>(
                    self,
                    __value: &[u8],
                ) -> _serde::__private::Result<Self::Value, __E>
                where
                    __E: _serde::de::Error,
                {
                    match __value {
                        _ => _serde::__private::Ok(__Field::__ignore),
                    }
                }
            }
            #[automatically_derived]
            impl<'de> _serde::Deserialize<'de> for __Field {
                #[inline]
                fn deserialize<__D>(
                    __deserializer: __D,
                ) -> _serde::__private::Result<Self, __D::Error>
                where
                    __D: _serde::Deserializer<'de>,
                {
                    _serde::Deserializer::deserialize_identifier(
                        __deserializer,
                        __FieldVisitor,
                    )
                }
            }
            #[doc(hidden)]
            struct __Visitor<'de> {
                marker: _serde::__private::PhantomData<GetCurrTime>,
                lifetime: _serde::__private::PhantomData<&'de ()>,
            }
            #[automatically_derived]
            impl<'de> _serde::de::Visitor<'de> for __Visitor<'de> {
                type Value = GetCurrTime;
                fn expecting(
                    &self,
                    __formatter: &mut _serde::__private::Formatter,
                ) -> _serde::__private::fmt::Result {
                    _serde::__private::Formatter::write_str(
                        __formatter,
                        "struct GetCurrTime",
                    )
                }
                #[inline]
                fn visit_seq<__A>(
                    self,
                    _: __A,
                ) -> _serde::__private::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::SeqAccess<'de>,
                {
                    _serde::__private::Ok(GetCurrTime {})
                }
                #[inline]
                fn visit_map<__A>(
                    self,
                    mut __map: __A,
                ) -> _serde::__private::Result<Self::Value, __A::Error>
                where
                    __A: _serde::de::MapAccess<'de>,
                {
                    while let _serde::__private::Some(__key) = _serde::de::MapAccess::next_key::<
                        __Field,
                    >(&mut __map)? {
                        match __key {
                            _ => {
                                let _ = _serde::de::MapAccess::next_value::<
                                    _serde::de::IgnoredAny,
                                >(&mut __map)?;
                            }
                        }
                    }
                    _serde::__private::Ok(GetCurrTime {})
                }
            }
            #[doc(hidden)]
            const FIELDS: &'static [&'static str] = &[];
            _serde::Deserializer::deserialize_struct(
                __deserializer,
                "GetCurrTime",
                FIELDS,
                __Visitor {
                    marker: _serde::__private::PhantomData::<GetCurrTime>,
                    lifetime: _serde::__private::PhantomData,
                },
            )
        }
    }
};
impl server_fn::ServerFn for GetCurrTime {
    const PATH: &'static str = ::const_format::pmr::__AssertStr {
        x: {
            use ::const_format::__cf_osRcTFl4A;
            ({
                #[doc(hidden)]
                #[allow(unused_mut, non_snake_case)]
                const CONCATP_NHPMWYD3NJA: &[__cf_osRcTFl4A::pmr::PArgument] = {
                    let fmt = __cf_osRcTFl4A::pmr::FormattingFlags::NEW;
                    &[
                        __cf_osRcTFl4A::pmr::PConvWrapper("/api")
                            .to_pargument_display(fmt),
                        __cf_osRcTFl4A::pmr::PConvWrapper("").to_pargument_display(fmt),
                        __cf_osRcTFl4A::pmr::PConvWrapper("/get_curr_time")
                            .to_pargument_display(fmt),
                    ]
                };
                {
                    #[doc(hidden)]
                    const ARR_LEN: usize = ::const_format::pmr::PArgument::calc_len(
                        CONCATP_NHPMWYD3NJA,
                    );
                    #[doc(hidden)]
                    const CONCAT_ARR: &::const_format::pmr::LenAndArray<[u8; ARR_LEN]> = &::const_format::pmr::__priv_concatenate(
                        CONCATP_NHPMWYD3NJA,
                    );
                    #[doc(hidden)]
                    #[allow(clippy::transmute_ptr_to_ptr)]
                    const CONCAT_STR: &str = unsafe {
                        let slice = ::const_format::pmr::transmute::<
                            &[u8; ARR_LEN],
                            &[u8; CONCAT_ARR.len],
                        >(&CONCAT_ARR.array);
                        {
                            let bytes: &'static [::const_format::pmr::u8] = slice;
                            let string: &'static ::const_format::pmr::str = {
                                ::const_format::__hidden_utils::PtrToRef {
                                    ptr: bytes as *const [::const_format::pmr::u8] as *const str,
                                }
                                    .reff
                            };
                            string
                        }
                    };
                    CONCAT_STR
                }
            })
        },
    }
        .x;
    type Client = server_fn::client::browser::BrowserClient;
    type Server = server_fn::mock::BrowserMockServer;
    type Protocol = server_fn::Http<server_fn::codec::PostUrl, server_fn::codec::Json>;
    type Output = String;
    type Error = ServerFnError;
    type InputStreamError = ServerFnError;
    type OutputStreamError = ServerFnError;
    fn middlewares() -> Vec<
        std::sync::Arc<
            dyn server_fn::middleware::Layer<
                <Self::Server as server_fn::server::Server<Self::Error>>::Request,
                <Self::Server as server_fn::server::Server<Self::Error>>::Response,
            >,
        >,
    > {
        ::alloc::vec::Vec::new()
    }
    #[allow(unused_variables)]
    async fn run_body(self) -> Result<String, ServerFnError> {
        ::core::panicking::panic("internal error: entered unreachable code")
    }
}
#[allow(unused_variables)]
async fn get_curr_time() -> Result<String, ServerFnError> {
    use server_fn::ServerFn;
    let data = GetCurrTime {};
    data.run_on_client().await
}
///Properties for the [`EvalIt`] component.
#[allow(missing_docs)]
#[allow(non_camel_case_types)]
struct EvalItProps {
    color: String,
}
impl EvalItProps {
    /**
Create a builder for building `EvalItProps`.
On the builder, call `.color(...)` to set the values of the fields.
Finally, call `.build()` to create the instance of `EvalItProps`.
                    */
    #[allow(dead_code, clippy::type_complexity)]
    fn builder() -> EvalItPropsBuilder<((),)> {
        EvalItPropsBuilder {
            fields: ((),),
            _phantom: ::core::default::Default::default(),
        }
    }
}
#[must_use]
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
struct EvalItPropsBuilder<TypedBuilderFields> {
    fields: TypedBuilderFields,
    _phantom: (),
}
impl dioxus_core::prelude::Properties for EvalItProps
where
    Self: Clone,
{
    type Builder = EvalItPropsBuilder<((),)>;
    fn builder() -> Self::Builder {
        EvalItProps::builder()
    }
    fn memoize(&mut self, new: &Self) -> bool {
        let equal = self == new;
        if !equal {
            let new_clone = new.clone();
            self.color = new_clone.color;
        }
        equal
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub trait EvalItPropsBuilder_Optional<T> {
    fn into_value<F: FnOnce() -> T>(self, default: F) -> T;
}
impl<T> EvalItPropsBuilder_Optional<T> for () {
    fn into_value<F: FnOnce() -> T>(self, default: F) -> T {
        default()
    }
}
impl<T> EvalItPropsBuilder_Optional<T> for (T,) {
    fn into_value<F: FnOnce() -> T>(self, _: F) -> T {
        self.0
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl EvalItPropsBuilder<((),)> {
    #[allow(clippy::type_complexity)]
    pub fn color(
        self,
        color: impl ::core::fmt::Display,
    ) -> EvalItPropsBuilder<((String,),)> {
        let color = (color.to_string(),);
        let (_,) = self.fields;
        EvalItPropsBuilder {
            fields: (color,),
            _phantom: self._phantom,
        }
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub enum EvalItPropsBuilder_Error_Repeated_field_color {}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl EvalItPropsBuilder<((String,),)> {
    #[deprecated(note = "Repeated field color")]
    #[allow(clippy::type_complexity)]
    pub fn color(
        self,
        _: EvalItPropsBuilder_Error_Repeated_field_color,
    ) -> EvalItPropsBuilder<((String,),)> {
        self
    }
}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, non_snake_case)]
pub enum EvalItPropsBuilder_Error_Missing_required_field_color {}
#[doc(hidden)]
#[allow(dead_code, non_camel_case_types, missing_docs, clippy::panic)]
impl EvalItPropsBuilder<((),)> {
    #[deprecated(note = "Missing required field color")]
    pub fn build(
        self,
        _: EvalItPropsBuilder_Error_Missing_required_field_color,
    ) -> EvalItProps {
        {
            #[cold]
            #[track_caller]
            #[inline(never)]
            const fn panic_cold_explicit() -> ! {
                ::core::panicking::panic_explicit()
            }
            panic_cold_explicit();
        }
    }
}
#[allow(dead_code, non_camel_case_types, missing_docs)]
impl EvalItPropsBuilder<((String,),)> {
    pub fn build(self) -> EvalItProps {
        let (color,) = self.fields;
        let color = color.0;
        EvalItProps { color }
    }
}
impl ::core::clone::Clone for EvalItProps {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            color: ::core::clone::Clone::clone(&self.color),
        }
    }
}
impl ::core::cmp::PartialEq for EvalItProps {
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        self.color == other.color && true
    }
}
/**# Props
*For details, see the [props struct definition](EvalItProps).**/
///- [`color`](EvalItProps::color) : `String`
#[allow(non_snake_case)]
fn EvalIt(EvalItProps { mut color }: EvalItProps) -> Element {
    {
        dioxus_core::internal::verify_component_called_as_component(EvalIt);
        {
            dioxus_core::Element::Ok({
                fn __original_template() -> &'static dioxus_core::internal::HotReloadedTemplate {
                    static __ORIGINAL_TEMPLATE: ::std::sync::OnceLock<
                        dioxus_core::internal::HotReloadedTemplate,
                    > = ::std::sync::OnceLock::new();
                    if __ORIGINAL_TEMPLATE.get().is_none() {
                        _ = __ORIGINAL_TEMPLATE
                            .set(
                                dioxus_core::internal::HotReloadedTemplate::new(
                                    None,
                                    <[_]>::into_vec(
                                        ::alloc::boxed::box_new([
                                            dioxus_core::internal::HotReloadDynamicNode::Dynamic(0usize),
                                        ]),
                                    ),
                                    <[_]>::into_vec(
                                        ::alloc::boxed::box_new([
                                            dioxus_core::internal::HotReloadDynamicAttribute::Dynamic(
                                                0usize,
                                            ),
                                        ]),
                                    ),
                                    ::alloc::vec::Vec::new(),
                                    __TEMPLATE_ROOTS,
                                ),
                            );
                    }
                    __ORIGINAL_TEMPLATE.get().unwrap()
                }
                let __template_read = {
                    static __NORMALIZED_FILE: &'static str = {
                        const PATH: &str = ::const_format::pmr::__AssertStr {
                            x: {
                                const ARGS_OSRCTFL4A: ::const_format::__str_methods::ReplaceInput = ::const_format::__str_methods::ReplaceInputConv(
                                        "examples/simple-spa-fullstack/src/main.rs",
                                        "\\\\",
                                        "/",
                                    )
                                    .conv();
                                {
                                    const OB: &[::const_format::pmr::u8; ARGS_OSRCTFL4A
                                        .replace_length()] = &ARGS_OSRCTFL4A.replace();
                                    const OS: &::const_format::pmr::str = unsafe {
                                        {
                                            let bytes: &'static [::const_format::pmr::u8] = OB;
                                            let string: &'static ::const_format::pmr::str = {
                                                ::const_format::__hidden_utils::PtrToRef {
                                                    ptr: bytes as *const [::const_format::pmr::u8] as *const str,
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
                                const ARGS_OSRCTFL4A: ::const_format::__str_methods::ReplaceInput = ::const_format::__str_methods::ReplaceInputConv(
                                        PATH,
                                        '\\',
                                        "/",
                                    )
                                    .conv();
                                {
                                    const OB: &[::const_format::pmr::u8; ARGS_OSRCTFL4A
                                        .replace_length()] = &ARGS_OSRCTFL4A.replace();
                                    const OS: &::const_format::pmr::str = unsafe {
                                        {
                                            let bytes: &'static [::const_format::pmr::u8] = OB;
                                            let string: &'static ::const_format::pmr::str = {
                                                ::const_format::__hidden_utils::PtrToRef {
                                                    ptr: bytes as *const [::const_format::pmr::u8] as *const str,
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
                        92u32,
                        5u32,
                        0usize,
                    );
                    dioxus_core::Runtime::current().ok().map(|_| __TEMPLATE.read())
                };
                let __template_read = match __template_read
                    .as_ref()
                    .map(|__template_read| __template_read.as_ref())
                {
                    Some(Some(__template_read)) => &__template_read,
                    _ => __original_template(),
                };
                let mut __dynamic_literal_pool = dioxus_core::internal::DynamicLiteralPool::new(
                    <[_]>::into_vec(
                        ::alloc::boxed::box_new([
                            ::alloc::__export::must_use({
                                    let res = ::alloc::fmt::format(format_args!("{0}", color));
                                    res
                                })
                                .to_string(),
                        ]),
                    ),
                );
                let __dynamic_nodes: [dioxus_core::DynamicNode; 1usize] = [
                    dioxus_core::DynamicNode::Text(
                        dioxus_core::VText::new(
                            ::alloc::__export::must_use({
                                let res = ::alloc::fmt::format(
                                    format_args!("eval -> {0}", color),
                                );
                                res
                            }),
                        ),
                    ),
                ];
                let __dynamic_attributes: [Box<[dioxus_core::Attribute]>; 1usize] = [
                    Box::new([
                        {
                            dioxus_elements::events::onclick::call_with_explicit_closure(move |
                                _|
                            {
                                _ = dioxus::document::eval(
                                    &::alloc::__export::must_use({
                                        let res = ::alloc::fmt::format(
                                            format_args!(
                                                "window.document.body.style.backgroundColor = \'{0}\';",
                                                color,
                                            ),
                                        );
                                        res
                                    }),
                                );
                            })
                        },
                    ]),
                ];
                #[doc(hidden)]
                static __TEMPLATE_ROOTS: &[dioxus_core::TemplateNode] = &[
                    {
                        dioxus_core::TemplateNode::Element {
                            tag: dioxus_elements::elements::div::TAG_NAME,
                            namespace: dioxus_elements::div::NAME_SPACE,
                            attrs: &[],
                            children: &[
                                {
                                    dioxus_core::TemplateNode::Element {
                                        tag: dioxus_elements::elements::button::TAG_NAME,
                                        namespace: dioxus_elements::button::NAME_SPACE,
                                        attrs: &[
                                            dioxus_core::TemplateAttribute::Dynamic {
                                                id: 0usize,
                                            },
                                        ],
                                        children: &[
                                            dioxus_core::TemplateNode::Dynamic {
                                                id: 0usize,
                                            },
                                        ],
                                    }
                                },
                            ],
                        }
                    },
                ];
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
mod EvalIt_completions {
    #[doc(hidden)]
    #[allow(non_camel_case_types)]
    /// This enum is generated to help autocomplete the braces after the component. It does nothing
    pub enum Component {
        EvalIt {},
    }
}
#[allow(unused)]
use EvalIt_completions::Component::EvalIt;
