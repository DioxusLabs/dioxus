Memos are great for deriving synchronous state, but sometimes you need to derive state that is asynchronous. In our previous example, we doubled the value of count. What if that doubling happened on the server? Instead of synchronously calling a function, we would need to start a request to the server and wait for it to finish and return a value.

Lets take a look at what that would look like in dioxus:

```rust
let count = use_state(|| 1);
let double_count = use_resource(move || async move {
    // Start a request to the server. We are reading the value of count to format it into the url
    // Since we are reading count, this resource will "subscribe" to changes to count (when count changes, the resource will rerun)
    let response = reqwest::get(format!("https://myserver.com/doubleme?count={count}")).await.unwrap();
    response.text().await.unwrap()
});

// Again, resources are similar to signals, but they have a bit of extra information. Unlike a memo, the resource may be in progress
// Calling .state() on a resource will return a Signal<UseResourceState> with information about the current status of the resource
println!("{:?}", double_count.state().read()); // Prints "UseResourceState::Pending"

// You can also try to get the last resolved value of the resource with the .value() method
println!("{:?}", double_count.value().read()); // Prints "None"

// Wait for the resource to finish and get the value
std::thread::sleep(std::time::Duration::from_secs(1));

// Now if we read the state, we will see that it is done
println!("{:?}", double_count.state().read()); // Prints "UseResourceState::Done"

// And we can get the value
println!("{:?}", double_count.value().read()); // Prints "Some(2)"

// Now if we write to count, the resource will rerun
count += 1; // count is now 2

// Wait for the resource to finish and get the value
std::thread::sleep(std::time::Duration::from_secs(1));

// Now if we read the state, we will see that it is done
println!("{:?}", double_count.state().read()); // Prints "UseResourceState::Done"

// And we can get the value
println!("{:?}", double_count.value().read()); // Prints "Some(4)"

// One more case, what happens if we write to the resource while it is in progress?
// The resource will rerun and the value will be None
count.write() += 1; // count is now 3

// If we write to a value the resource subscribes to again, it will cancel the current future and start a new one
count += 1; // count is now 4

println!("{:?}", double_count.state().read()); // Prints "UseResourceState::Stopped"
println!("{:?}", double_count.value().read()); // Prints the last resolved value "Some(4)"

// After we wait for the resource to finish, we will get the value of only the latest future
std::thread::sleep(std::time::Duration::from_secs(1));

println!("{:?}", double_count.state().read()); // Prints "UseResourceState::Done"

println!("{:?}", double_count.value().read()); // Prints "Some(8)"
```

> Note: I made some analogies to memo, but unlike memos, resources do not memorize the output of the closure. They will always rerun any parts of your app that read the value of the resource when the future resolves even if the output doesn't change.
