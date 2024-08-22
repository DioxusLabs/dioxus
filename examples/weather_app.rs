#![allow(non_snake_case)]

use dioxus::prelude::*;
use serde::{Deserialize, Serialize};

fn main() {
    launch(app);
}

fn app() -> Element {
    let country = use_signal(|| WeatherLocation {
        name: "Berlin".to_string(),
        country: "Germany".to_string(),
        latitude: 52.5244,
        longitude: 13.4105,
        id: 2950159,
    });

    let current_weather = use_resource(move || async move { get_weather(&country()).await });

    rsx! {
        document::Stylesheet { href: "https://unpkg.com/tailwindcss@^2.0/dist/tailwind.min.css" }
        div { class: "mx-auto p-4 bg-gray-100 h-screen flex justify-center",
            div { class: "flex items-center justify-center flex-row",
                div { class: "flex items-start justify-center flex-row",
                    SearchBox { country }
                    div { class: "flex flex-wrap w-full px-2",
                        div { class: "bg-gray-900 text-white relative min-w-0 break-words rounded-lg overflow-hidden shadow-sm mb-4 w-full dark:bg-gray-600",
                            div { class: "px-6 py-6 relative",
                                if let Some(Ok(weather)) = current_weather.read().as_ref() {
                                    CountryData {
                                        country: country.read().clone(),
                                        weather: weather.clone(),
                                    }
                                    Forecast { weather: weather.clone() }
                                } else {
                                    p { "Loading.." }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[allow(non_snake_case)]
#[component]
fn CountryData(weather: WeatherResponse, country: WeatherLocation) -> Element {
    let today = "Today";
    let max_temp = weather.daily.temperature_2m_max.first().unwrap();
    let min_temp = weather.daily.temperature_2m_min.first().unwrap();

    rsx! {
        div { class: "flex mb-4 justify-between items-center",
            div {
                h5 { class: "mb-0 font-medium text-xl", "{country.name} 🏞️" }
                h6 { class: "mb-0", "{today}" }
            }
            div {
                div { class: "flex items-center",
                    span { "Temp min" }
                    span { class: "px-2 inline-block", "👉 {min_temp}°" }
                }
                div { class: "flex items-center",
                    span { "Temp max" }
                    span { class: "px-2 inline-block ", "👉 {max_temp}º" }
                }
            }
        }
    }
}

#[allow(non_snake_case)]
#[component]
fn Forecast(weather: WeatherResponse) -> Element {
    let today = (weather.daily.temperature_2m_max.first().unwrap()
        + weather.daily.temperature_2m_max.first().unwrap())
        / 2.0;
    let tomorrow = (weather.daily.temperature_2m_max.get(1).unwrap()
        + weather.daily.temperature_2m_max.get(1).unwrap())
        / 2.0;
    let past_tomorrow = (weather.daily.temperature_2m_max.get(2).unwrap()
        + weather.daily.temperature_2m_max.get(2).unwrap())
        / 2.0;

    rsx! {
        div { class: "px-6 pt-4 relative",
            div { class: "w-full h-px bg-gray-100 mb-4" }
            div { p { class: "text-center w-full mb-4", "👇 Forecast 📆" } }
            div { class: "text-center justify-between items-center flex",
                div { class: "text-center mb-0 flex items-center justify-center flex-col mx-4 w-16",
                    span { class: "block my-1", "Today" }
                    span { class: "block my-1", "{today}°" }
                }
                div { class: "text-center mb-0 flex items-center justify-center flex-col mx-8 w-16",
                    span { class: "block my-1", "Tomorrow" }
                    span { class: "block my-1", "{tomorrow}°" }
                }
                div { class: "text-center mb-0 flex items-center justify-center flex-col mx-2 w-30",
                    span { class: "block my-1", "Past Tomorrow" }
                    span { class: "block my-1", "{past_tomorrow}°" }
                }
            }
        }
    }
}

#[component]
fn SearchBox(mut country: Signal<WeatherLocation>) -> Element {
    let mut input = use_signal(|| "".to_string());

    let locations = use_resource(move || async move { get_locations(&input()).await });

    rsx! {
        div {
            div { class: "inline-flex flex-col justify-center relative text-gray-500",
                div { class: "relative",
                    input {
                        class: "p-2 pl-8 rounded-lg border border-gray-200 bg-gray-200 focus:bg-white focus:outline-none focus:ring-2 focus:ring-yellow-600 focus:border-transparent",
                        placeholder: "Country name",
                        "type": "text",
                        autofocus: true,
                        oninput: move |e: FormEvent| input.set(e.value())
                    }
                    svg {
                        class: "w-4 h-4 absolute left-2.5 top-3.5",
                        "viewBox": "0 0 24 24",
                        fill: "none",
                        stroke: "currentColor",
                        xmlns: "http://www.w3.org/2000/svg",
                        path {
                            d: "M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z",
                            "stroke-linejoin": "round",
                            "stroke-linecap": "round",
                            "stroke-width": "2"
                        }
                    }
                }
                ul { class: "bg-white border border-gray-100 w-full mt-2 max-h-72 overflow-auto",
                    if let Some(Ok(locs)) = locations.read().as_ref() {
                        for wl in locs.iter().take(5).cloned() {
                            li { class: "pl-8 pr-2 py-1 border-b-2 border-gray-100 relative cursor-pointer hover:bg-yellow-50 hover:text-gray-900",
                                onclick: move |_| country.set(wl.clone()),
                                MapIcon {}
                                b { "{wl.name}" }
                                " · {wl.country}"
                            }
                        }
                    } else {
                        "loading locations..."
                    }
                }
            }
        }
    }
}

fn MapIcon() -> Element {
    rsx! {
        svg {
            class: "stroke-current absolute w-4 h-4 left-2 top-2",
            stroke: "currentColor",
            xmlns: "http://www.w3.org/2000/svg",
            "viewBox": "0 0 24 24",
            fill: "none",
            path {
                "stroke-linejoin": "round",
                "stroke-width": "2",
                "stroke-linecap": "round",
                d: "M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z"
            }
            path {
                "stroke-linecap": "round",
                "stroke-linejoin": "round",
                d: "M15 11a3 3 0 11-6 0 3 3 0 016 0z",
                "stroke-width": "2"
            }
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone)]
struct WeatherLocation {
    id: usize,
    name: String,
    latitude: f32,
    longitude: f32,
    country: String,
}

type WeatherLocations = Vec<WeatherLocation>;

#[derive(Debug, Default, Serialize, Deserialize)]
struct SearchResponse {
    results: WeatherLocations,
}

async fn get_locations(input: &str) -> reqwest::Result<WeatherLocations> {
    let res = reqwest::get(&format!(
        "https://geocoding-api.open-meteo.com/v1/search?name={input}"
    ))
    .await?
    .json::<SearchResponse>()
    .await?;

    Ok(res.results)
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone)]
struct WeatherResponse {
    daily: DailyWeather,
    hourly: HourlyWeather,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone)]
struct HourlyWeather {
    time: Vec<String>,
    temperature_2m: Vec<f32>,
}

#[derive(Debug, Default, Serialize, Deserialize, PartialEq, Clone)]
struct DailyWeather {
    temperature_2m_min: Vec<f32>,
    temperature_2m_max: Vec<f32>,
}

async fn get_weather(location: &WeatherLocation) -> reqwest::Result<WeatherResponse> {
    let res = reqwest::get(&format!("https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&hourly=temperature_2m&daily=temperature_2m_max,temperature_2m_min,apparent_temperature_max,apparent_temperature_min&timezone=GMT", location.latitude, location.longitude))
        .await
        ?
        .json::<WeatherResponse>()
        .await
        ?;

    Ok(res)
}
