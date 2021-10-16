//! Example: Weather App
//! --------------------
//!
//!
//!

use dioxus::prelude::*;

fn main() {
    dioxus::desktop::launch(App, |c| c);
}

const ENDPOINT: &str = "https://api.openweathermap.org/data/2.5/weather";

static App: FC<()> = |(cx, props)| {
    //
    let body = use_suspense(
        cx,
        || async {
            let content = reqwest::get(ENDPOINT)
                .await
                .unwrap()
                .json::<serde_json::Value>()
                .await
                .unwrap();
        },
        |cx, props| {
            //
            rsx!(cx, WeatherDisplay {})
        },
    );

    rsx!(cx, div {
        {body}
    })
};

#[derive(PartialEq, Props)]
struct WeatherProps {}

static WeatherDisplay: FC<WeatherProps> = |(cx, props)| {
    //
    cx.render(rsx!(
        div { class: "flex items-center justify-center flex-col"
            div { class: "flex items-center justify-center"
                div { class: "flex flex-col bg-white rounded p-4 w-full max-w-xs"
                    div{ class: "font-bold text-xl"
                        "Jon's awesome site!!"
                    }
                    div{ class: "text-sm text-gray-500"
                        "He worked so hard on it :)"
                    }
                    div { class: "flex flex-row items-center justify-center mt-6"
                        div { class: "font-medium text-6xl"
                            "1337"
                        }
                    }
                    div { class: "flex flex-row justify-between mt-6"
                        "Legit made my own React"
                    }
                }
            }
        }
    ))
};
