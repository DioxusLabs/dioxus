use const_serialize::{serialize_eq, SerializeConst};

#[derive(Clone, Copy, Debug, PartialEq, SerializeConst)]
struct Struct {
    a: u32,
    b: u8,
    c: u32,
    d: Enum,
}

#[derive(Clone, Copy, Debug, PartialEq, SerializeConst)]
#[repr(C, u8)]
enum Enum {
    A { one: u32, two: u16 },
    B { one: u8, two: u16 } = 15,
}

#[test]
fn const_eq() {
    const {
        let data = [
            Struct {
                a: 0x11111111,
                b: 0x22,
                c: 0x33333333,
                d: Enum::A {
                    one: 0x44444444,
                    two: 0x5555,
                },
            },
            Struct {
                a: 123,
                b: 9,
                c: 38,
                d: Enum::B {
                    one: 0x44,
                    two: 0x555,
                },
            },
            Struct {
                a: 9,
                b: 123,
                c: 39,
                d: Enum::B {
                    one: 0x46,
                    two: 0x555,
                },
            },
        ];
        let mut other = data;
        other[2].a += 1;
        if serialize_eq(&data, &other) {
            panic!("data should be different");
        }
        if !serialize_eq(&data, &data) {
            panic!("data should be the same");
        }
    }
}
