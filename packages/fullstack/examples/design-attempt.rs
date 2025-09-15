/*
parse out URL params
rest need to implement axum's FromRequest / extract
body: String
body: Bytes
payload: T where T: Deserialize (auto to Json, can wrap in other codecs)
extra items get merged as body, unless theyre also extractors?
hoist up FromRequest objects if they're just bounds
no State<T> extractors, use ServerState instead?

if there's a single trailing item, it's used as the body?

or, an entirely custom system, maybe based on names?
or, hoist up FromRequest objects into the signature?
*/

/*

an fn that returns an IntoFuture / async fn
- is clearer that it's an async fn....
- still shows up as a function
- can guard against being called on the client with IntoFuture?
- can be used as a handler directly
- requires a trait to be able to mess with it
- codegen for handling inputs seems more straightforward?

a static that implements Deref to a function pointer
- can guard against being called on the client
- can be used as a handler directly
- has methods on the static itself (like .path(), .method()) as well as the result
- does not show up as a proper function in docs
- callable types are a weird thing to do. deref is always weird to overload
- can have a builder API!

qs:
- should we even make it so you can access its props directly?
*/

fn main() {}
