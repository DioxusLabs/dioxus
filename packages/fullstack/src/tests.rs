use super::*;
use crate::codec::JsonEncoding;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
enum TestError {
    ServerFnError(ServerFnErrorErr),
}

impl FromServerFnError for TestError {
    type Encoder = JsonEncoding;

    fn from_server_fn_error(value: ServerFnErrorErr) -> Self {
        Self::ServerFnError(value)
    }
}

#[test]
fn test_result_serialization() {
    // Test Ok variant
    let ok_result: Result<Bytes, Bytes> = Ok(Bytes::from_static(b"success data"));
    let serialized = serialize_result(ok_result);
    let deserialized = deserialize_result::<TestError>(serialized);
    assert!(deserialized.is_ok());
    assert_eq!(deserialized.unwrap(), Bytes::from_static(b"success data"));

    // Test Err variant
    let err_result: Result<Bytes, Bytes> = Err(Bytes::from_static(b"error details"));
    let serialized = serialize_result(err_result);
    let deserialized = deserialize_result::<TestError>(serialized);
    assert!(deserialized.is_err());
    assert_eq!(
        deserialized.unwrap_err(),
        Bytes::from_static(b"error details")
    );
}
