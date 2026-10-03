fn attributes() -> Element {
    rsx! {
		div {
			class: "a", // after the only attribute
		}
		div {
			class: "a", // after a
			id: "b", // after b, which has no comma
		}
		div {
			// above
			class: "a",
		}
		div {
			class: "a",
			// after the last attribute
		}
		div { class: "a", // after
			"child"
		}
		div { class: "a", id: "b", // after b
			div {}
			div {}
		}
		div {
			class: "a",

			// above b, after several blank lines
			id: "b",
		}
		div {
			class: "a",
			// above
			..attrs,
		}
		div {
			class: "a",
			..attrs, // after
		}
		div {
			// above
			..attrs,
		}
		Comp {
			a: 1, // after a
			// above the spread
			..props, // after the spread
			// after everything
		}
		div {
			class: "w-1/4",
			onclick: move |_| {
			    todo!();
			    todo!();
				// at the end of a closure
			},
			"Name"
		}
	}
}

fn control_flow() -> Element {
    rsx! {
		for i in 0..10 { // after the opening brace
			// above the child
			div { "{i}" } // after the child
			// before the closing brace
		} // after the closing brace
		for i in 0..10 {
			// only a comment
		}
		for i in 0..10 { // after the opening brace of an empty loop
		}
		if a { // after the opening brace of the if
			// above the child of the if
			div { "a" }
			// before the closing brace of the if
		} else if b { // after the opening brace of the else if
			// only a comment
		} else { // after the opening brace of the else
			div { "c" }

			// before the closing brace of the else
		} // after the closing brace of the chain
		if a {} else {}
		div {
			for a in b {
				if a {
					// deep inside
				}
				// before the closing brace of the loop
			}
			// before the closing brace of the element
		}
	}
}

fn positions() -> Element {
    rsx! { // after the opening brace
		div { // after the opening brace
			"child" // after the child
		} // after the closing brace
		div {
			// only comments

			// separated by blank lines
		}
		Comp {
			// only a comment
		}
		div {
			"a" // after a short child
		}
		div {
			// above a short child
			"a"
		}
		div { class: "🦀", // after an emoji
			"héllo 🦀" // after text with an emoji
		}
		// after the last root
	}
}

fn only_comment() -> Element {
    rsx! {
		// only a comment
	}
}

fn after_open_text() -> Element {
    rsx! { // after the opening brace
		"text"
	}
}
