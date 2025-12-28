use const_serialize::{deserialize_const, serialize_const, ConstVec};

#[test]
fn test_serialize_const_layout_primitive() {
    let mut buf = ConstVec::new();
    buf = serialize_const(&1234u32, buf);
    let buf = buf.as_ref();
    assert_eq!(deserialize_const!(u32, buf).unwrap().1, 1234u32);

    let mut buf = ConstVec::new();
    buf = serialize_const(&1234u64, buf);
    let buf = buf.as_ref();
    assert_eq!(deserialize_const!(u64, buf).unwrap().1, 1234u64);

    let mut buf = ConstVec::new();
    buf = serialize_const(&1234i32, buf);
    let buf = buf.as_ref();
    assert_eq!(deserialize_const!(i32, buf).unwrap().1, 1234i32);

    let mut buf = ConstVec::new();
    buf = serialize_const(&1234i64, buf);
    let buf = buf.as_ref();
    assert_eq!(deserialize_const!(i64, buf).unwrap().1, 1234i64);

    let mut buf = ConstVec::new();
    buf = serialize_const(&true, buf);
    assert_eq!(buf.as_ref(), [1u8]);
    let buf = buf.as_ref();
    assert!(deserialize_const!(bool, buf).unwrap().1);

    let mut buf = ConstVec::new();
    buf = serialize_const(&0.631f32, buf);
    let buf = buf.as_ref();
    assert_eq!(deserialize_const!(f32, buf).unwrap().1, 0.631);
}

#[test]

fn test_serialize_primitive_too_little_data() {
    let mut buf = ConstVec::new();
    buf = buf.push(1);
    buf = buf.push(1);
    buf = buf.push(1);
    let buf = buf.as_ref();
    assert_eq!(deserialize_const!([u64; 10], buf), None);
}
