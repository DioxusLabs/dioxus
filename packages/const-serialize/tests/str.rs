use const_serialize::{deserialize_const, serialize_const, ConstStr, ConstWriteBuffer};

#[test]
fn test_serialize_const_layout_str() {
    let mut buf = ConstWriteBuffer::new();
    let str = ConstStr::new("hello");
    buf = serialize_const(&str, buf);
    println!("{:?}", buf.as_ref());
    let buf = buf.read();
    assert_eq!(deserialize_const!(ConstStr, buf).unwrap().as_str(), "hello");
}

#[test]
fn test_serialize_const_layout_nested_str() {
    let mut buf = ConstWriteBuffer::new();
    let str = ConstStr::new("hello");
    buf = serialize_const(&[str, str, str] as &[ConstStr; 3], buf);
    println!("{:?}", buf.as_ref());
    let buf = buf.read();

    assert_eq!(
        deserialize_const!([ConstStr; 3], buf),
        Some([
            ConstStr::new("hello"),
            ConstStr::new("hello"),
            ConstStr::new("hello")
        ])
    );
}

#[test]
fn test_serialize_str_too_little_data() {
    let mut buf = ConstWriteBuffer::new();
    buf = buf.push(1);
    let buf = buf.read();
    assert_eq!(deserialize_const!(ConstStr, buf), None);
}