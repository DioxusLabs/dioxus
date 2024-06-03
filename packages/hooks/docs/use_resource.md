[`use_resource()`] is a reactive hook that resolves to the result of a future. It will rerun when you write to any signals you read inside the future.

## Example

```rust
use dioxus::prelude::*;

async fn get_weather(location: &WeatherLocation) -> Result<String, String> {
    Ok("Sunny".to_string())
}

fn app() -> Element {
    let country = use_signal(|| WeatherLocation {
        city: "Berlin".to_string(),
        country: "Germany".to_string(),
        coordinates: (52.5244, 13.4105),
    });

    // Because the resource's future subscribes to `country` by reading it (`country.read()`),
    // every time `country` changes the resource's future will run again and thus provide a new value.
    let current_weather = use_resource(move || async move { get_weather(&country()).await });

    rsx! {
        // the value of the resource can be polled to
        // conditionally render elements based off if it's future
        // finished (Some(Ok(_)), errored Some(Err(_)),
        // or is still running (None)
        match &*current_weather.read_unchecked() {
            Some(Ok(weather)) => rsx! { WeatherElement { weather } },
            Some(Err(e)) => rsx! { p { "Loading weather failed, {e}" } },
            None =>  rsx! { p { "Loading..." } }
        }
    }
}

#[derive(Clone)]
struct WeatherLocation {
    city: String,
    country: String,
    coordinates: (f64, f64),
}

#[component]
fn WeatherElement(weather: String) -> Element {
    rsx! { p { "The weather is {weather}" } }
}
```

## Reactivity

`use_resource` is reactive which just means that it will rerun when you write to any signals you read inside the future. This means that any time you change something the future depends on, the resource automatically knows to rerun. Lets take a look at some examples:

```rust, no_run
# use dioxus::prelude::*;
// Create a new count signal
let mut count = use_signal(|| 1);
// Create a new resource that doubles the value of count
let double_count = use_resource(move || async move {
    // Start a request to the server. We are reading the value of count in the format macro
    // Reading the value of count makes the resource "subscribe" to changes to count (when count changes, the resource will rerun)
    let response = reqwest::get(format!("https://myserver.com/doubleme?count={count}")).await.unwrap();
    response.text().await.unwrap()
});

// Resource can be read in a way that is similar to signals, but they have a bit of extra information about the state of the resource future.

// Calling .state() on a resource will return a Signal<UseResourceState> with information about the current status of the resource
println!("{:?}", double_count.state().read()); // Prints "UseResourceState::Pending"

// You can also try to get the last resolved value of the resource with the .value() method
println!("{:?}", double_count.read()); // Prints "None"

// Wait for the resource to finish and get the value
std::thread::sleep(std::time::Duration::from_secs(1));

// Now if we read the state, we will see that it is done
println!("{:?}", double_count.state().read()); // Prints "UseResourceState::Done"

// And we can get the value
println!("{:?}", double_count.read()); // Prints "Some(2)"

// Now if we write to count, the resource will rerun
count += 1; // count is now 2

// Wait for the resource to finish and get the value
std::thread::sleep(std::time::Duration::from_secs(1));

// Now if we read the state, we will see that it is done
println!("{:?}", double_count.state().read()); // Prints "UseResourceState::Done"

// And we can get the value
println!("{:?}", double_count.read()); // Prints "Some(4)"

// One more case, what happens if we write to the resource while it is in progress?
// The resource will rerun and the value will be None
count += 1; // count is now 3

// If we write to a value the resource subscribes to again, it will cancel the current future and start a new one
count += 1; // count is now 4

println!("{:?}", double_count.state().read()); // Prints "UseResourceState::Stopped"
println!("{:?}", double_count.read()); // Prints the last resolved value "Some(4)"

// After we wait for the resource to finish, we will get the value of only the latest future
std::thread::sleep(std::time::Duration::from_secs(1));

println!("{:?}", double_count.state().read()); // Prints "UseResourceState::Done"

println!("{:?}", double_count.read()); // Prints "Some(8)"
```

## With non-reactive dependencies

`use_resource` can determine dependencies automatically with any reactive value ([`Signal`]s, [`ReadOnlySignal`]s, [`Memo`]s, [`Resource`]s, etc). If you need to rerun the future when a normal rust value changes, you can add it as a dependency with the [`crate::use_reactive()`] hook:

```rust
# use dioxus::prelude::*;
# async fn sleep(delay: u32) {}
#[component]
fn Comp(count: u32) -> Element {
    // We manually add the resource to the dependencies list with the `use_reactive` hook
    // Any time `count` changes, the resource will rerun
    let new_count = use_resource(use_reactive!(|(count,)| async move {
        sleep(100).await;
        count + 1
    }));
    rsx! { "{new_count:?}" }
}

// If your value is already reactive, you never need to call `use_reactive` manually
// Instead of manually adding count to the dependencies list, you can make your prop reactive by wrapping it in `ReadOnlySignal`
#[component]
fn ReactiveComp(count: ReadOnlySignal<u32>) -> Element {
    // Because `count` is reactive, the resource knows to rerun when `count` changes automatically
    let new_count = use_resource(move || async move {
        sleep(100).await;
        count() + 1
    });
    rsx! { "{new_count:?}" }
}
```

## Differences from `use_future` and `use_memo`

Just like [`crate::use_future()`], `use_resource` spawns an async task in a component. However, unlike [`crate::use_future()`], `use_resource` returns the result of the future and will rerun when any dependencies change.

Resources return a value based on some existing state just like memos, but unlike memos, resources do not memorize the output of the closure. They will always rerun any parts of your app that read the value of the resource when the future resolves even if the output doesn't change.

See also: [`Resource`]
