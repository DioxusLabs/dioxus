#![allow(non_upper_case_globals)]

use dioxus_ssr::{prelude::*, TextRenderer};
pub mod utils;
mod components {
    mod community;
    pub use community::*;
    mod docs;
    pub use docs::*;
    mod blog;
    pub use blog::*;
    mod homepage;
    pub use homepage::*;
    mod tutorial;
    pub use tutorial::*;
}
use components::*;

fn main() {
    let renderer = TextRenderer::new(App);
    /*
    renderer.render_at("community")

    */
}

static App: FC<()> = |cx| {
    let (url, set_url) = use_state(&cx, || "");

    let body = match *url {
        "community" => rsx!(in cx, Community {}),
        "tutorial" => rsx!(in cx, Tutorial {}),
        "blog" => rsx!(in cx, Blog {}),
        "docs" => rsx!(in cx, Docs {}),
        _ => rsx!(in cx, Home {}),
    };

    cx.render(rsx! {
        div {
            NavBar {}
            {body}
            Footer {}
        }
    })
};

static NavBar: FC<()> = |cx| {
    cx.render(rsx! {
        header {
            a {
                href: "/"
                img { /*logo*/ }
                span {}
            }
            nav {
                a { href: "/community/support", "Community" }
                a { href: "/docs/getting-started", "Docs" }
                a { href: "/tutorial/tutorial", "Tutorial" }
                a { href: "/blog/", "Blog" }
            }
            form {}
            div {}
        }
    })
};

static SECTIONS: &[(&str, &[(&str, &str)])] = &[
    (
        "Docs",
        &[
            ("Installation", "docs/installation"),
            ("Main Concepts", "docs/main"),
            ("Advanced Guides", "docs/advanced"),
            ("Hooks", "docs/hooks"),
            ("Testing", "docs/testing"),
            ("Contributing", "docs/contributing"),
            ("FAQ", "docs/faq"),
        ],
    ),
    (
        "Channels",
        &[("Github", "https://github.com/jkelleyrtp/dioxus")],
    ),
    (
        "Community",
        &[
            ("Code of Conduct", "docs/installation"),
            ("Community Resources", "docs/main"),
        ],
    ),
    (
        "More",
        &[
            ("Tutorial", "docs/installation"),
            ("Blog", "docs/main"),
            ("Privacy", "docs/advanced"),
            ("Terms", "docs/hooks"),
        ],
    ),
];

fn Footer(cx: Context<()>) -> VNode {
    let sections = SECTIONS.iter().map(|(section, raw_links)| {
        let links = raw_links.iter().map(|(link_name, href)| {
            rsx! (
                a { href: "{href}",
                    "{link_name}",
                    {href.starts_with("http").then(|| rsx!( ExternalLinkIcon {} ))}
                }
            )
        });
        rsx! {
            div {
                div { "{section}" }
                {links}
            }
        }
    });

    cx.render(rsx! {
        footer {
            div {
                div {
                    div {
                        {sections}
                    }
                    section {
                        a {
                            img {}
                        }
                        p {}
                    }
                }
            }
        }
    })
}

const ExternalLinkIcon: FC<()> = |cx| {
    cx.render(html! {
        <svg x="0px" y="0px" viewBox="0 0 100 100" width="15" height="15">
            <path
                fill="currentColor"
                d="M18.8,85.1h56l0,0c2.2,0,4-1.8,4-4v-32h-8v28h-48v-48h28v-8h-32l0, 0c-2.2,0-4,1.8-4,4v56C14.8,83.3,16.6,85.1,18.8,85.1z"
            ></path>
            <polygon
                fill="currentColor"
                points="45.7,48.7 51.3,54.3 77.2,28.5 77.2,37.2 85.2,37.2 85.2,14.9 62.8,14.9 62.8,22.9 71.5,22.9"
            ></polygon>
        </svg>
    })
};
