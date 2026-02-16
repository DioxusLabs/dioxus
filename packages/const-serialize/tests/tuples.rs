use const_serialize::{deserialize_const, serialize_const, ConstVec};

#[test]
fn test_serialize_const_layout_tuple() {
    let mut buf = ConstVec::new();
    buf = serialize_const(&(1234u32, 5678u16), buf);
    let buf = buf.as_ref();
    assert_eq!(
        deserialize_const!((u32, u16), buf).unwrap().1,
        (1234u32, 5678u16)
    );

    let mut buf = ConstVec::new();
    buf = serialize_const(&(1234f64, 5678u16, 90u8), buf);
    let buf = buf.as_ref();
    assert_eq!(
        deserialize_const!((f64, u16, u8), buf).unwrap().1,
        (1234f64, 5678u16, 90u8)
    );

    let mut buf = ConstVec::new();
    buf = serialize_const(&(1234u32, 5678u16, 90u8, 1000000f64), buf);
    let buf = buf.as_ref();
    assert_eq!(
        deserialize_const!((u32, u16, u8, f64), buf).unwrap().1,
        (1234u32, 5678u16, 90u8, 1000000f64)
    );
}
