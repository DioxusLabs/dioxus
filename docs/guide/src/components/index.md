# Introduction to Components

In the previous chapter, we learned about Elements and how they can be composed to create a basic user interface. Now, we'll learn how to group Elements together to form Components. We'll cover:

- What makes a Component
- How to model a component and its properties in Dioxus
- How to "think declaratively"

## What is a component?

In short, a component is a special function that takes input properties and outputs an Element. Much like a function encapsulates some specific computation task, a Component encapsulates some specific rendering task – typically, rendering an isolated part of the user interface.

### Real-world example

Take a look at a post on Reddit:

![Reddit Post](../images/reddit_post.png)

We can build this post as a component, consisting of multiple subcomponents. It has several inputs:

- The score
- The number of comments
- How long ago it was posted
- The url short address
- The title
- The username of the original poster

If we wanted to sketch out these requirements in Rust, we would start with a struct:

```rust
struct PostData {
    score: i32,
    comment_count: u32,
    post_time: Instant,
    url: String,
    title: String,
    original_poster_name: String
}
```

If we look at the layout of the component, we notice quite a few buttons and pieces of functionality:

- Upvote/Downvote
- View comments
- Share
- Save
- Hide
- Give award
- Report
- Crosspost
- Filter by site
- View article
- Visit user

If we included all this functionality in one `rsx!` call it would be huge! Instead, let's break the post down into some core pieces:

![Post as Component](../images/reddit_post_components.png)

- **VoteButton**: Upvote/Downvote
- **TitleCard**: Title, Filter-By-Url
- **MetaCard**: Original Poster, Time Submitted
- **ActionCard**: View comments, Share, Save, Hide, Give award, Report, Crosspost

In this chapter, we'll learn how to define our own components.
