use const_serialize::{deserialize_const, serialize_const, ConstVec, SerializeConst};
use std::mem::MaybeUninit;

#[test]
fn test_transmute_bytes_to_enum() {
    #[derive(Clone, Copy, Debug, PartialEq)]
    #[repr(C, u8)]
    enum Enum<T> {
        A { one: u32, two: u16 },
        B { one: u8, two: T } = 15,
    }

    #[repr(C)]
    #[derive(Debug, PartialEq)]
    struct A {
        one: u32,
        two: u16,
    }

    #[repr(C)]
    #[derive(Debug, PartialEq)]
    struct B<T> {
        one: u8,
        two: T,
    }

    const SIZE: usize = std::mem::size_of::<Enum<u16>>();
    let mut out = [MaybeUninit::uninit(); SIZE];
    let discriminate_size = std::mem::size_of::<u8>();
    let tag_align = 0;
    let union_alignment = std::mem::align_of::<A>().max(std::mem::align_of::<B<u16>>());
    let data_align = (discriminate_size / union_alignment) + union_alignment;
    let a_one_align = std::mem::offset_of!(A, one);
    let a_two_align = std::mem::offset_of!(A, two);
    let b_one_align = std::mem::offset_of!(B<u16>, one);
    let b_two_align = std::mem::offset_of!(B<u16>, two);

    let one = 1234u32;
    let two = 5678u16;
    let first = Enum::A { one, two };
    for (i, byte) in one.to_le_bytes().iter().enumerate() {
        out[data_align + i + a_one_align] = MaybeUninit::new(*byte);
    }
    for (i, byte) in two.to_le_bytes().iter().enumerate() {
        out[data_align + i + a_two_align] = MaybeUninit::new(*byte);
    }
    out[tag_align] = MaybeUninit::new(0);
    let out = unsafe { std::mem::transmute_copy::<[MaybeUninit<u8>; SIZE], Enum<u16>>(&out) };
    assert_eq!(out, first);

    let mut out = [MaybeUninit::uninit(); SIZE];
    let one = 123u8;
    let two = 58u16;
    let second = Enum::B { one, two };
    for (i, byte) in one.to_le_bytes().iter().enumerate() {
        out[data_align + i + b_one_align] = MaybeUninit::new(*byte);
    }
    for (i, byte) in two.to_le_bytes().iter().enumerate() {
        out[data_align + i + b_two_align] = MaybeUninit::new(*byte);
    }
    out[tag_align] = MaybeUninit::new(15);
    let out = unsafe { std::mem::transmute_copy::<[MaybeUninit<u8>; SIZE], Enum<u16>>(&out) };
    assert_eq!(out, second);
}

#[test]
fn test_serialize_enum() {
    #[derive(Clone, Copy, Debug, PartialEq, SerializeConst)]
    #[repr(C, u8)]
    enum Enum {
        A { one: u32, two: u16 },
        B { one: u8, two: u16 } = 15,
    }

    println!("{:#?}", Enum::MEMORY_LAYOUT);

    let data = Enum::A {
        one: 0x11111111,
        two: 0x22,
    };
    let mut buf = ConstVec::new();
    buf = serialize_const(&data, buf);
    println!("{:?}", buf.as_ref());
    let buf = buf.read();
    assert_eq!(deserialize_const!(Enum, buf).unwrap().1, data);

    let data = Enum::B {
        one: 0x11,
        two: 0x2233,
    };
    let mut buf = ConstVec::new();
    buf = serialize_const(&data, buf);
    println!("{:?}", buf.as_ref());
    let buf = buf.read();
    assert_eq!(deserialize_const!(Enum, buf).unwrap().1, data);
}

#[test]
fn test_serialize_u8_enum() {
    #[derive(Clone, Copy, Debug, PartialEq, SerializeConst)]
    #[repr(u8)]
    enum Enum {
        A,
        B,
    }

    println!("{:#?}", Enum::MEMORY_LAYOUT);

    let data = Enum::A;
    let mut buf = ConstVec::new();
    buf = serialize_const(&data, buf);
    println!("{:?}", buf.as_ref());
    let buf = buf.read();
    assert_eq!(deserialize_const!(Enum, buf).unwrap().1, data);

    let data = Enum::B;
    let mut buf = ConstVec::new();
    buf = serialize_const(&data, buf);
    println!("{:?}", buf.as_ref());
    let buf = buf.read();
    assert_eq!(deserialize_const!(Enum, buf).unwrap().1, data);
}

#[test]
fn test_serialize_corrupted_enum() {
    #[derive(Clone, Copy, Debug, PartialEq, SerializeConst)]
    #[repr(C, u8)]
    enum Enum {
        A { one: u32, two: u16 },
    }

    let data = Enum::A {
        one: 0x11111111,
        two: 0x22,
    };
    let mut buf = ConstVec::new();
    buf = serialize_const(&data, buf);
    buf = buf.set(0, 2);
    println!("{:?}", buf.as_ref());
    let buf = buf.read();
    assert_eq!(deserialize_const!(Enum, buf), None);
}

#[test]
fn test_serialize_nested_enum() {
    #[derive(Clone, Copy, Debug, PartialEq, SerializeConst)]
    #[repr(C, u8)]
    enum Enum {
        A { one: u32, two: u16 },
        B { one: u8, two: InnerEnum } = 15,
    }

    #[derive(Clone, Copy, Debug, PartialEq, SerializeConst)]
    #[repr(C, u16)]
    enum InnerEnum {
        A(u8),
        B { one: u64, two: f64 } = 1000,
        C { one: u32, two: u16 },
    }

    let data = Enum::A {
        one: 0x11111111,
        two: 0x22,
    };
    let mut buf = ConstVec::new();
    buf = serialize_const(&data, buf);
    println!("{:?}", buf.as_ref());
    let buf = buf.read();
    assert_eq!(deserialize_const!(Enum, buf).unwrap().1, data);

    let data = Enum::B {
        one: 0x11,
        two: InnerEnum::A(0x22),
    };
    let mut buf = ConstVec::new();
    buf = serialize_const(&data, buf);
    println!("{:?}", buf.as_ref());
    let buf = buf.read();
    assert_eq!(deserialize_const!(Enum, buf).unwrap().1, data);

    let data = Enum::B {
        one: 0x11,
        two: InnerEnum::B {
            one: 0x2233,
            two: 0.123456789,
        },
    };
    let mut buf = ConstVec::new();
    buf = serialize_const(&data, buf);
    println!("{:?}", buf.as_ref());
    let buf = buf.read();
    assert_eq!(deserialize_const!(Enum, buf).unwrap().1, data);

    let data = Enum::B {
        one: 0x11,
        two: InnerEnum::C {
            one: 0x2233,
            two: 56789,
        },
    };
    let mut buf = ConstVec::new();
    buf = serialize_const(&data, buf);
    println!("{:?}", buf.as_ref());
    let buf = buf.read();
    assert_eq!(deserialize_const!(Enum, buf).unwrap().1, data);
}
