A rust serialization library that works in const with complex(ish) types like enums, nested structs and arrays. Const rust does not have an allocator, so this library cannot work in a cross architecture environment with Vecs, slices or strings.

```rust
use const_serialize::{deserialize_const, serialize_const, serialize_eq, ConstVec, SerializeConst};
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

const {
    let data = [Struct {
        a: 0x11111111,
        b: 0x22,
        c: 0x33333333,
        d: Enum::A {
            one: 0x44444444,
            two: 0x5555,
        },
    }; 3];
    let mut buf = ConstVec::new();
    buf = serialize_const(&data, buf);
    let buf = buf.as_ref();
    let (buf, deserialized) = match deserialize_const!([Struct; 3], buf) {
        Some(data) => data,
        None => panic!("data mismatch"),
    };
    if !serialize_eq(&data, &deserialized) {
        panic!("data mismatch");
    }
}
```

## How it works

`const-serialize` relies heavily on well defined layouts for the types you want to serialize. The serialization format is the linear sequence of unaligned bytes stored in the order of the fields, items or variants of the type. Numbers are stored in little endian order.

In order to support complex nested types, serialization is done using a trait. Since functions in traits cannot be const, `const-serialize` uses a macro to generate constant associated items that describe the memory layout of the type. That layout is then used to read all of the bytes in the type into the serialized buffer.

The deserialization is done in a similar way, but the layout is used to write the bytes from the serialized buffer into the type.

The rust [nomicon](https://doc.rust-lang.org/nomicon/data.html) defines the memory layout of different types. It is used as a reference for the layout of the types implemented in `const-serialize`.

## Limitations

- Only constant sized types are supported. This means that you can't serialize a type like `Vec<T>`. These types are difficult to create in const contexts in general
- Only types with a well defined memory layout are supported (see <https://github.com/rust-lang/rfcs/pull/3727> and <https://onevariable.com/blog/pods-from-scratch>). `repr(Rust)` enums don't have a well defined layout, so they are not supported. `repr(C, u8)` enums can be used instead
- Const rust does not support mutable references or points, so this crate leans heavily on functional data structures for data processing.
