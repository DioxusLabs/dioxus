rsx! {
    // Before a tag
    <div
        // Before an attribute
        class="a"
        id // After an attribute
    >
        // Before the first child
        <h1>"Hello"</h1> // After a tag

        <img src="image.png" /> // After a self-closing tag
        // Before the closing tag
    </div>
    <section>
        // Only a comment
    </section>
    <button
        onclick={move |_| {
            println!("clicked");
        }}
    >
        "Click"
    </button>
}
