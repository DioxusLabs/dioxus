#![allow(non_snake_case, unused)]
use dioxus::prelude::*;
// Define the Hackernews API and types
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::{
    fmt::{Display, Formatter},
    num::ParseIntError,
    str::FromStr,
};
use svg_attributes::to;

fn main() {
    #[cfg(feature = "web")]
    tracing_wasm::set_as_global_default();

    #[cfg(feature = "server")]
    tracing_subscriber::fmt::init();

    launch(|| rsx! { Router::<Route> {} });
}

#[derive(Clone, Routable)]
enum Route {
    #[redirect("/", || Route::Homepage { story: PreviewState { active_story: None } })]
    #[route("/:story")]
    Homepage { story: PreviewState },
}

pub fn App() -> Element {
    rsx! {
        Router::<Route> {}
    }
}

#[component]
fn Homepage(story: ReadOnlySignal<PreviewState>) -> Element {
    rsx! {
        document::Stylesheet { href: asset!("/assets/hackernews.css") }
        div { display: "flex", flex_direction: "row", width: "100%",
            div {
                width: "50%",
                SuspenseBoundary {
                    fallback: |context: SuspenseContext| rsx! {
                        "Loading..."
                    },
                    Stories {}
                }
            }
            div { width: "50%",
                SuspenseBoundary {
                    fallback: |context: SuspenseContext| rsx! {
                        "Loading preview..."
                    },
                    Preview {
                        story
                    }
                }
            }
        }
    }
}

#[component]
fn Stories() -> Element {
    let stories: Resource<dioxus::Result<Vec<i64>>> = use_server_future(move || async move {
        let url = format!("{}topstories.json", BASE_API_URL);
        let mut stories_ids = reqwest::get(&url).await?.json::<Vec<i64>>().await?;
        stories_ids.truncate(30);
        Ok(stories_ids)
    })?;

    match stories().unwrap() {
        Ok(list) => rsx! {
            div {
                for story in list {
                    ChildrenOrLoading {
                        key: "{story}",
                        StoryListing { story }
                    }
                }
            }
        },
        Err(err) => rsx! {"An error occurred while fetching stories {err}"},
    }
}

#[component]
fn StoryListing(story: ReadOnlySignal<i64>) -> Element {
    let story = use_server_future(move || get_story(story()))?;

    let StoryItem {
        title,
        url,
        by,
        score,
        time,
        kids,
        id,
        ..
    } = story().unwrap()?.item;

    let url = url.as_deref().unwrap_or_default();
    let hostname = url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_start_matches("www.");
    let score = format!("{score} {}", if score == 1 { " point" } else { " points" });
    let comments = format!(
        "{} {}",
        kids.len(),
        if kids.len() == 1 {
            " comment"
        } else {
            " comments"
        }
    );
    let time = time.format("%D %l:%M %p");

    rsx! {
        div {
            padding: "0.5rem",
            position: "relative",
            div { font_size: "1.5rem",
                Link {
                    to: Route::Homepage { story: PreviewState { active_story: Some(id) } },
                    "{title}"
                }
                a {
                    color: "gray",
                    href: "https://news.ycombinator.com/from?site={hostname}",
                    text_decoration: "none",
                    " ({hostname})"
                }
            }
            div { display: "flex", flex_direction: "row", color: "gray",
                div { "{score}" }
                div { padding_left: "0.5rem", "by {by}" }
                div { padding_left: "0.5rem", "{time}" }
                div { padding_left: "0.5rem", "{comments}" }
            }
        }
    }
}

#[derive(Clone, Debug, Default)]
struct PreviewState {
    active_story: Option<i64>,
}

impl FromStr for PreviewState {
    type Err = ParseIntError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let state = i64::from_str(s)?;
        Ok(PreviewState {
            active_story: Some(state),
        })
    }
}

impl Display for PreviewState {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        if let Some(id) = &self.active_story {
            write!(f, "{id}")?;
        }
        Ok(())
    }
}

#[component]
fn Preview(story: ReadOnlySignal<PreviewState>) -> Element {
    let PreviewState {
        active_story: Some(id),
    } = story()
    else {
        return rsx! {"Hover over a story to preview it here"};
    };

    let story = use_server_future(use_reactive!(|id| get_story(id)))?;

    let story = story().unwrap()?;

    rsx! {
        div { padding: "0.5rem",
            div { font_size: "1.5rem", a { href: story.item.url, "{story.item.title}" } }
            if let Some(text) = &story.item.text { div { dangerous_inner_html: "{text}" } }
            for comment in story.item.kids.iter().copied() {
                ChildrenOrLoading {
                    key: "{comment}",
                    Comment { comment }
                }
            }
        }
    }
}

#[component]
fn Comment(comment: i64) -> Element {
    let comment: Resource<dioxus::Result<CommentData>> =
        use_server_future(use_reactive!(|comment| async move {
            let url = format!("{}{}{}.json", BASE_API_URL, ITEM_API, comment);
            let mut comment = reqwest::get(&url).await?.json::<CommentData>().await?;
            Ok(comment)
        }))?;

    let CommentData {
        by,
        time,
        text,
        id,
        kids,
        ..
    } = comment().unwrap()?;

    rsx! {
        div { padding: "0.5rem",
            div { color: "gray", "by {by}" }
            div { dangerous_inner_html: "{text}" }
            for comment in kids.iter().copied() {
                ChildrenOrLoading {
                    key: "{comment}",
                    Comment { comment }
                }
            }
        }
    }
}

pub static BASE_API_URL: &str = "https://hacker-news.firebaseio.com/v0/";
pub static ITEM_API: &str = "item/";
pub static USER_API: &str = "user/";
const COMMENT_DEPTH: i64 = 1;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StoryPageData {
    #[serde(flatten)]
    pub item: StoryItem,
    #[serde(default)]
    pub comments: Vec<CommentData>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CommentData {
    pub id: i64,
    /// there will be no by field if the comment was deleted
    #[serde(default)]
    pub by: String,
    #[serde(default)]
    pub text: String,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub time: DateTime<Utc>,
    #[serde(default)]
    pub kids: Vec<i64>,
    pub r#type: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct StoryItem {
    pub id: i64,
    pub title: String,
    pub url: Option<String>,
    pub text: Option<String>,
    #[serde(default)]
    pub by: String,
    #[serde(default)]
    pub score: i64,
    #[serde(default)]
    pub descendants: i64,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub time: DateTime<Utc>,
    #[serde(default)]
    pub kids: Vec<i64>,
    pub r#type: String,
}

pub async fn get_story(id: i64) -> dioxus::Result<StoryPageData> {
    let url = format!("{}{}{}.json", BASE_API_URL, ITEM_API, id);
    Ok(reqwest::get(&url).await?.json::<StoryPageData>().await?)
}

#[component]
fn ChildrenOrLoading(children: Element) -> Element {
    rsx! {
        SuspenseBoundary {
            fallback: |context: SuspenseContext| {
                rsx! {
                    if let Some(placeholder) = context.suspense_placeholder() {
                        {placeholder}
                    } else {
                        LoadingIndicator {}
                    }
                }
            },
            children
        }
    }
}

fn LoadingIndicator() -> Element {
    rsx! {
        div {
            class: "spinner",
        }
    }
}
