pub static Icon3: Component<()> = |cx| {
    rsx! {
        svg {
            class: "w-6 h-6",
            stroke_linecap: "round",
            fill: "none",
            stroke_linejoin: "round",
            stroke_width: "2",
            stroke: "currentColor",
            view_box: "0 0 24 24",
            path { d: "M20 21v-2a4 4 0 00-4-4H8a4 4 0 00-4 4v2" }
            circle { cx: "12", cy: "7", r: "4" }
        }
    }
};
