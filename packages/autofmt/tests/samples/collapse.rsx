// nesting pushes out
rsx! {
    Fragment {
        Fragment {
            Fragment {
                Fragment {
                    Fragment {
                        div { "Finally have a real node!" }
                    }
                }
            }
        }
    }
}

// we don't make extra spaces
rsx! {
    Component { blah: rsx! {} }
}

rsx! {}
