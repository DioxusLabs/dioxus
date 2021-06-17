# Asynchronous state management

Async is pervasive in web frontends. The web heavily relies on external resources like http, requests, storage, etc.

## How does Redux do it?

Thunks

- call a synchronous dispatch function
- queues an asynchronous "thunk"
- thunks may not modify state, may only call reducers

## How does Recoil do it?

useRecoilCallback

- call an async function
- get/set any atom with the Recoil API
- synchronously set values

async atoms

- atom's "get" returns a promise that resolves to the atom's value
- if a request fails

## How should we do it?

```rust
const USER_ID: Atom<String> = |atom| String::from("ThisIsDummyData");
const TITLE: Selector<Result<String>> = |atom| {
    atom.async_get(|api| async {
        let user_id = api.get(ID);
        let resp: Result<UserData> = surf::get(format!("https://myapi.com/users/{}", user_id))
            .recv_json()
            .await;
    });
}

#[derive(Deref)]
struct UserManager(RecoilApi);
impl UserManager {
    fn load_user_data() {

    }
}

async fn Example(cx: Context<()>) -> Result<VNode> {
    let title = use_read(cx, TITLE)?;
}
```

```rust
fn use_initialize_app() {
    // initialize theme systems
    // initialize websocket connection
    // create task queue
}
```
