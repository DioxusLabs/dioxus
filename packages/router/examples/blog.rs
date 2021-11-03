use dioxus::prelude::*;

fn main() {}

enum Routes {
    Home,
    Blog,
}

static App: FC<()> = |(cx, props)| {
    let route = use_router::<Routes>(cx);

    let content = match route {
        Routes::Home => rsx!(Home {}),
        Routes::Blog => rsx!(Blog {}),
    };

    cx.render(rsx! {
        {content}
    })
};

fn use_router<P>(cx: Context) -> &P {
    todo!()
}

static Home: FC<()> = |(cx, props)| todo!();
static Blog: FC<()> = |(cx, props)| todo!();
