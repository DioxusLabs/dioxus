use bytes::Bytes;
use dioxus_fullstack::req_to::{EncodeRequest, EncodeState, ReqSer};

#[tokio::main]
async fn main() {
    // queries, url get passed through ctx
    let serializer = (&&&&&&&&&&&&&&ReqSer::<(i32, i32, Bytes)>::new())
        .encode::<String>(EncodeState::default(), (1, 2, Bytes::from("hello")))
        .await
        .unwrap();
}
