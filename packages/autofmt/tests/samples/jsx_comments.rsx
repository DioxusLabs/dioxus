rsx! {
    // Before a tag
    <div
        // Before an attribute
        class="a"
        id={user_id} // After an attribute
        onclick={move |_| {
            // Inside a closure
            println!("clicked");
        }} // After a closure
        // Before a spread
        {..attrs} // After a spread
    >
        // Before the first child
        <h1>"Hello"</h1> // After a tag

        <img src="image.png" /> // After a self-closing tag
        <p> // After an open tag
            "text"
        </p>
        // Before a block
        span { "block" } // After a block
        // Before the closing tag
    </div>
    <section>
        // Only a comment
    </section>
    div {
        // Before a tag in a block
        <b>"bold"</b>

        <i>"italic"</i>
        // Before the closing brace
    }
    <section>
        // Only comments

        // Separated by a blank line
    </section>
    <ul>
        for i in 0..3 { // After the opening brace of a loop
            // Before a tag in a loop
            <li>"{i}"</li> // After a tag in a loop
            // Before the closing brace of a loop
        }
        if a {
            // Only a comment
        } else { // After the opening brace of an else
            <b /> // After a self-closing tag in an else
        }
    </ul>
    <div class="🦀"> // After an open tag with an emoji
        "héllo 🦀" // After text with an emoji
    </div>
    <img
        // Before the only attribute
        src="a" // After the only attribute
    />
}
