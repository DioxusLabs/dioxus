# This crate autofmts blocks of rsx!

This crate formats rsx! by parsing call bodies and pretty-printing them back out.



# Todo:
Sorted roughly in order of what's possible

- [x] Oneline rsx! calls - blocker because this wrecks formatting
- [ ] Nested RSX calls (important) - unnecessary but desirable
- [ ] RSX edits overstepping each other
- [ ] Collapse components and elements under syntax -
- [ ] Don't eat comments in exprs
- [ ] Format regular exprs
- [ ] Fix prettyplease around chaining
- [ ] Don't eat comments in prettyplease


# Technique


div {
    div {}
    div {}
}


div

possible line break
div
div



string of possible items within a nesting
div {
    attr_pair
    expr
    text
    comment
}
a nesting is either a component or an element

idea:
collect all items into a queue
q
```rust
section {
    div {
        h1 { p { "asdasd" } }
        h1 { p { "asdasd" } }
    }
}

section {}
```


// space
// space
// space


3 - section
3 - section div
3 - section div h1
3 - section div h1 p
3 - section div h1 p text
3 - section
3 - section div
3 - section div h1
3 - section div h1 p
3 - section div h1 p text

block

- when we hit the end of a trail, we can make a decision what needs to be hard breaked
- most nestings cannot be merged into a single one, so at some point we need to write the line break
- this is the scan section. we scan forward until it's obvious where to place a hard break
- when a line is finished, we can print it out by unloading our queued items
- never double nested


Terms
- break is a whitespace than can flex, dependent on the situation
- ‹⁠›
