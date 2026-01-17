use const_serialize::{deserialize_const, serialize_const, ConstVec};

#[test]
fn test_serialize_const_layout_list() {
    let mut buf = ConstVec::new();
    buf = serialize_const(&[1u8, 2, 3] as &[u8; 3], buf);
    println!("{:?}", buf.as_ref());
    let buf = buf.as_ref();
    assert_eq!(deserialize_const!([u8; 3], buf).unwrap().1, [1, 2, 3])
}

#[test]
fn test_serialize_const_layout_nested_lists() {
    let mut buf = ConstVec::new();
    buf = serialize_const(
        &[[1u8, 2, 3], [4u8, 5, 6], [7u8, 8, 9]] as &[[u8; 3]; 3],
        buf,
    );
    println!("{:?}", buf.as_ref());
    let buf = buf.as_ref();

    assert_eq!(
        deserialize_const!([[u8; 3]; 3], buf).unwrap().1,
        [[1, 2, 3], [4, 5, 6], [7, 8, 9]]
    );
}

#[test]
fn test_serialize_list_too_little_data() {
    let mut buf = ConstVec::new();
    buf = buf.push(1);
    let buf = buf.as_ref();
    assert_eq!(deserialize_const!([u64; 10], buf), None);
}
