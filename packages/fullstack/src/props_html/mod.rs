pub(crate) mod deserialize_props;

pub(crate) mod serialize_props;

#[test]
fn serialized_and_deserializes() {
    use postcard::to_allocvec;

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
    struct Data {
        a: u32,
        b: String,
        bytes: Vec<u8>,
        nested: Nested,
    }

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq, Clone)]
    struct Nested {
        a: u32,
        b: u16,
        c: u8,
    }

    for x in 0..10usize {
        for y in 0..10 {
            let mut as_string = String::new();
            let data = vec![
                Data {
                    a: x as u32,
                    b: "hello".to_string(),
                    bytes: vec![0; x],
                    nested: Nested {
                        a: 1,
                        b: x as u16,
                        c: 3
                    },
                };
                y
            ];
            serialize_props::serde_to_writable(&data, &mut unsafe { as_string.as_bytes_mut() })
                .unwrap();

            println!("{}", as_string);
            println!(
                "original size: {}",
                std::mem::size_of::<Data>() * data.len()
            );
            println!("serialized size: {}", to_allocvec(&data).unwrap().len());
            println!("compressed size: {}", as_string.len());

            let decoded: Vec<Data> = deserialize_props::serde_from_string(&as_string).unwrap();
            assert_eq!(data, decoded);
        }
    }
}
