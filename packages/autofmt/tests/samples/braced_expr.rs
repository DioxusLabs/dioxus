fn main() {
    {
        POSTS.iter().enumerate().map(|(id, post)| {
            rsx! {
                BlogPostItem { post, id }
            }
        })
    }
}
