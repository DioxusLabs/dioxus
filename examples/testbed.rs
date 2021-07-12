use dioxus::prelude::*;

fn main() {
    env_logger::init();
    dioxus::desktop::launch(Example, |c| c);
}

const STYLE: &str = r#"
body {background-color: powderblue;}
h1   {color: blue;}
p    {color: red;}
"#;

const Example: FC<()> = |cx| {
    cx.render(rsx! {
        Child { }
        Child { }
    })
};

const Child: FC<()> = |cx| {
    cx.render(rsx!(
        h1 {"1" }
        h1 {"2" }
        h1 {"3" }
        h1 {"4" }
    ))
};
