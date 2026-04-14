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
    LaunchBuilder::new()
        .with_cfg(server_only! {
            dioxus::server::ServeConfig::builder().enable_out_of_order_streaming()
        })
        .launch(|| {
            rsx! {
                Stylesheet { href: asset!("/assets/hackernews.css") }
                Router::<Route> {}
            }
        });
}

#[derive(Clone, Routable)]
enum Route {
    #[route("/story&:story")]
    StoryPreview { story: Option<i64> },
}

#[component]
fn StoryPreview(story: ReadSignal<Option<i64>>) -> Element {
    rsx! {
        div { display: "flex", flex_direction: "row", width: "100%",
            div { width: "50%",
                SuspenseBoundary { fallback: |context| rsx! { "Loading..." },
                    Stories {}
                }
            }
            div { width: "50%",
                SuspenseBoundary { fallback: |context| rsx! { "Loading preview..." },
                    if let Some(story) = story() {
                        Preview { story_id: story }
                    } else {
                        div { padding: "0.5rem", "Select a story to preview" }
                    }
                }
            }
        }
    }
}

#[component]
fn Stories() -> Element {
    let stories = use_loader(move || async move {
        let stories_ids = reqwest::get(&format!("{}topstories.json", BASE_API_URL))
            .await?
            .json::<Vec<i64>>()
            .await?
            .into_iter()
            .take(30)
            .collect::<Vec<i64>>();
        dioxus::Ok(stories_ids)
    })?;

    rsx! {
        div {
            for story in stories() {
                ChildrenOrLoading { key: "{story}",
                    StoryListing { story }
                }
            }
        }
    }
}

#[component]
fn StoryListing(story: ReadSignal<i64>) -> Element {
    let story = use_loader(move || get_story(story()))?;
    let StoryItem {
        title,
        url,
        by,
        score,
        time,
        kids,
        id,
        ..
    } = story().item;

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
                    to: Route::StoryPreview { story: Some(id) },
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

#[component]
fn Preview(story_id: ReadSignal<i64>) -> Element {
    let story = use_loader(move || get_story(story_id()))?.cloned();
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
fn Comment(comment: ReadSignal<i64>) -> Element {
    let comment = use_loader(move || async move {
        let mut comment = reqwest::get(&format!("{}{}{}.json", BASE_API_URL, ITEM_API, comment))
            .await?
            .json::<CommentData>()
            .await?;
        dioxus::Ok(comment)
    })?;

    let CommentData {
        by,
        time,
        text,
        id,
        kids,
        ..
    } = comment();

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

pub async fn get_story(id: i64) -> Result<StoryPageData> {
    Ok(
        reqwest::get(&format!("{}{}{}.json", BASE_API_URL, ITEM_API, id))
            .await?
            .json::<StoryPageData>()
            .await?,
    )
}

#[component]
fn ChildrenOrLoading(children: Element) -> Element {
    rsx! {
        SuspenseBoundary {
            fallback: |_| rsx! { div { class: "spinner", } },
            children
        }
    }
}
