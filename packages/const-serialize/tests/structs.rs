use const_serialize::{deserialize_const, serialize_const, ConstVec, SerializeConst};
use std::mem::MaybeUninit;

#[test]
fn test_transmute_bytes_to_struct() {
    struct MyStruct {
        a: u32,
        b: u8,
        c: u32,
        d: u32,
    }
    const SIZE: usize = std::mem::size_of::<MyStruct>();
    let mut out = [MaybeUninit::uninit(); SIZE];
    let first_align = std::mem::offset_of!(MyStruct, a);
    let second_align = std::mem::offset_of!(MyStruct, b);
    let third_align = std::mem::offset_of!(MyStruct, c);
    let fourth_align = std::mem::offset_of!(MyStruct, d);
    for (i, byte) in 1234u32.to_le_bytes().iter().enumerate() {
        out[i + first_align] = MaybeUninit::new(*byte);
    }
    for (i, byte) in 12u8.to_le_bytes().iter().enumerate() {
        out[i + second_align] = MaybeUninit::new(*byte);
    }
    for (i, byte) in 13u32.to_le_bytes().iter().enumerate() {
        out[i + third_align] = MaybeUninit::new(*byte);
    }
    for (i, byte) in 14u32.to_le_bytes().iter().enumerate() {
        out[i + fourth_align] = MaybeUninit::new(*byte);
    }
    let out = unsafe { std::mem::transmute_copy::<[MaybeUninit<u8>; SIZE], MyStruct>(&out) };
    assert_eq!(out.a, 1234);
    assert_eq!(out.b, 12);
    assert_eq!(out.c, 13);
    assert_eq!(out.d, 14);
}

#[test]
fn test_serialize_const_layout_struct_list() {
    #[derive(Clone, Copy, Debug, PartialEq, SerializeConst)]
    struct Struct {
        a: u32,
        b: u8,
        c: u32,
        d: u32,
    }

    impl Struct {
        #[allow(dead_code)]
        const fn equal(&self, other: &Struct) -> bool {
            self.a == other.a && self.b == other.b && self.c == other.c && self.d == other.d
        }
    }

    #[derive(Clone, Copy, Debug, PartialEq, SerializeConst)]
    struct OtherStruct {
        a: u32,
        b: u8,
        c: Struct,
        d: u32,
    }

    impl OtherStruct {
        #[allow(dead_code)]
        const fn equal(&self, other: &OtherStruct) -> bool {
            self.a == other.a && self.b == other.b && self.c.equal(&other.c) && self.d == other.d
        }
    }

    const INNER_DATA: Struct = Struct {
        a: 0x11111111,
        b: 0x22,
        c: 0x33333333,
        d: 0x44444444,
    };
    const DATA: [OtherStruct; 3] = [
        OtherStruct {
            a: 0x11111111,
            b: 0x22,
            c: INNER_DATA,
            d: 0x44444444,
        },
        OtherStruct {
            a: 0x111111,
            b: 0x23,
            c: INNER_DATA,
            d: 0x44444444,
        },
        OtherStruct {
            a: 0x11111111,
            b: 0x11,
            c: INNER_DATA,
            d: 0x44441144,
        },
    ];

    const _ASSERT: () = {
        let mut buf = ConstVec::new();
        buf = serialize_const(&DATA, buf);
        let buf = buf.as_ref();
        let [first, second, third] = match deserialize_const!([OtherStruct; 3], buf) {
            Some((_, data)) => data,
            None => panic!("data mismatch"),
        };
        if !(first.equal(&DATA[0]) && second.equal(&DATA[1]) && third.equal(&DATA[2])) {
            panic!("data mismatch");
        }
    };
    const _ASSERT_2: () = {
        let mut buf = ConstVec::new();
        const DATA_AGAIN: [[OtherStruct; 3]; 3] = [DATA, DATA, DATA];
        buf = serialize_const(&DATA_AGAIN, buf);
        let buf = buf.as_ref();
        let [first, second, third] = match deserialize_const!([[OtherStruct; 3]; 3], buf) {
            Some((_, data)) => data,
            None => panic!("data mismatch"),
        };
        if !(first[0].equal(&DATA[0]) && first[1].equal(&DATA[1]) && first[2].equal(&DATA[2])) {
            panic!("data mismatch");
        }
        if !(second[0].equal(&DATA[0]) && second[1].equal(&DATA[1]) && second[2].equal(&DATA[2])) {
            panic!("data mismatch");
        }
        if !(third[0].equal(&DATA[0]) && third[1].equal(&DATA[1]) && third[2].equal(&DATA[2])) {
            panic!("data mismatch");
        }
    };

    let mut buf = ConstVec::new();
    buf = serialize_const(&DATA, buf);
    println!("{:?}", buf.as_ref());
    let buf = buf.as_ref();
    let (_, data2) = deserialize_const!([OtherStruct; 3], buf).unwrap();
    assert_eq!(DATA, data2);
}

#[test]
fn test_serialize_const_layout_struct() {
    #[derive(Debug, PartialEq, SerializeConst)]
    struct Struct {
        a: u32,
        b: u8,
        c: u32,
        d: u32,
    }

    #[derive(Debug, PartialEq, SerializeConst)]
    struct OtherStruct(u32, u8, Struct, u32);

    println!("{:?}", OtherStruct::MEMORY_LAYOUT);

    let data = Struct {
        a: 0x11111111,
        b: 0x22,
        c: 0x33333333,
        d: 0x44444444,
    };
    let data = OtherStruct(0x11111111, 0x22, data, 0x44444444);
    let mut buf = ConstVec::new();
    buf = serialize_const(&data, buf);
    println!("{:?}", buf.as_ref());
    let buf = buf.as_ref();
    let (_, data2) = deserialize_const!(OtherStruct, buf).unwrap();
    assert_eq!(data, data2);
}

#[test]
fn test_adding_struct_field_non_breaking() {
    #[derive(Debug, PartialEq, SerializeConst)]
    struct Initial {
        a: u32,
        b: u8,
    }

    #[derive(Debug, PartialEq, SerializeConst)]
    struct New {
        c: u32,
        b: u8,
        a: u32,
    }

    let data = New {
        a: 0x11111111,
        b: 0x22,
        c: 0x33333333,
    };
    let mut buf = ConstVec::new();
    buf = serialize_const(&data, buf);
    let buf = buf.as_ref();
    // The new struct should be able to deserialize into the initial struct
    let (_, data2) = deserialize_const!(Initial, buf).unwrap();
    assert_eq!(
        Initial {
            a: data.a,
            b: data.b,
        },
        data2
    );
}
