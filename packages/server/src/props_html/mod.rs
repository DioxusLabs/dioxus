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
            serialize_props::serde_to_writable(&data, &mut as_string).unwrap();

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

#[test]
fn encodes_and_decodes_bytes() {
    for i in 0..(u16::MAX) {
        let c = u16_to_char(i);
        let i2 = u16_from_char(c);
        assert_eq!(i, i2);
    }
}

#[allow(unused)]
pub(crate) fn u16_to_char(u: u16) -> char {
    let u = u as u32;
    let mapped = if u <= 0xD7FF {
        u
    } else {
        0xE000 + (u - 0xD7FF)
    };
    char::from_u32(mapped).unwrap()
}

#[allow(unused)]
pub(crate) fn u16_from_char(c: char) -> u16 {
    let c = c as u32;
    let mapped = if c <= 0xD7FF {
        c
    } else {
        0xD7FF + (c - 0xE000)
    };
    mapped as u16
}
