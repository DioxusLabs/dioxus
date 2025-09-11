use bytes::Bytes;
use http::HeaderMap;
use serde::de::DeserializeSeed;

#[tokio::main]
async fn main() {
    let request = axum::extract::Request::new(axum::body::Body::empty());

    // let (a, b, c, d, e) = Extractor::extract(
    //     |x| (&&x).extract::<HeaderMap>(),
    //     |x| (&&x).extract::<String>(),
    //     |x| (&&x).extract::<i32>(),
    //     |x| (&&x).extract::<f64>(),
    //     |x| Nothing,
    //     request,
    // )
    // .await;

    // Extractor::new()
    //     .queue::<HeaderMap>()
    //     .queue::<String>()
    //     .queue::<i32>()
    //     .queue::<f64>()
    //     .queue::<Bytes>()
    //     .queue::<()>()
    //     .extract(request)
    //     .await;
}

// struct Extractor<TypeChain, Names> {
//     _phantom: std::marker::PhantomData<TypeChain>,
//     names: Names,
// }
// impl DeserializeSeed for Extractor<>
