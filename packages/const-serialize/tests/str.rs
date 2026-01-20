use const_serialize::{deserialize_const, serialize_const, ConstStr, ConstVec};

#[test]
fn test_serialize_const_layout_str() {
    let mut buf = ConstVec::new();
    let str = ConstStr::new("hello");
    buf = serialize_const(&str, buf);
    println!("{:?}", buf.as_ref());
    let buf = buf.as_ref();
    assert!(buf.len() < 10);
    let str = deserialize_const!(ConstStr, buf).unwrap().1;
    assert_eq!(str.as_str(), "hello");
}

#[test]
fn test_serialize_const_layout_nested_str() {
    let mut buf = ConstVec::new();
    let str = ConstStr::new("hello");
    buf = serialize_const(&[str, str, str] as &[ConstStr; 3], buf);
    println!("{:?}", buf.as_ref());
    assert!(buf.len() < 30);
    let buf = buf.as_ref();

    assert_eq!(
        deserialize_const!([ConstStr; 3], buf).unwrap().1,
        [
            ConstStr::new("hello"),
            ConstStr::new("hello"),
            ConstStr::new("hello")
        ]
    );
}

#[test]
fn test_serialize_str_too_little_data() {
    let mut buf = ConstVec::new();
    buf = buf.push(1);
    let buf = buf.as_ref();
    assert_eq!(deserialize_const!(ConstStr, buf), None);
}
