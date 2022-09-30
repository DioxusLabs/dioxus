use std::fmt::Formatter;

/// Possible values for an attribute
// trying to keep values at 3 bytes
#[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serialize", serde(untagged))]
#[derive(Clone, Debug, PartialEq)]
#[allow(missing_docs)]
pub enum AttributeValue<'a> {
    Text(&'a str),
    Float32(f32),
    Float64(f64),
    Int32(i32),
    Int64(i64),
    Uint32(u32),
    Uint64(u64),
    Bool(bool),

    Vec3Float(f32, f32, f32),
    Vec3Int(i32, i32, i32),
    Vec3Uint(u32, u32, u32),

    Vec4Float(f32, f32, f32, f32),
    Vec4Int(i32, i32, i32, i32),
    Vec4Uint(u32, u32, u32, u32),

    Bytes(&'a [u8]),
    Any(ArbitraryAttributeValue<'a>),
}

// todo
#[allow(missing_docs)]
impl<'a> AttributeValue<'a> {
    pub fn is_truthy(&self) -> bool {
        match self {
            AttributeValue::Text(t) => *t == "true",
            AttributeValue::Bool(t) => *t,
            _ => false,
        }
    }

    pub fn is_falsy(&self) -> bool {
        match self {
            AttributeValue::Text(t) => *t == "false",
            AttributeValue::Bool(t) => !(*t),
            _ => false,
        }
    }
}

impl<'a> std::fmt::Display for AttributeValue<'a> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            AttributeValue::Text(a) => write!(f, "{}", a),
            AttributeValue::Float32(a) => write!(f, "{}", a),
            AttributeValue::Float64(a) => write!(f, "{}", a),
            AttributeValue::Int32(a) => write!(f, "{}", a),
            AttributeValue::Int64(a) => write!(f, "{}", a),
            AttributeValue::Uint32(a) => write!(f, "{}", a),
            AttributeValue::Uint64(a) => write!(f, "{}", a),
            AttributeValue::Bool(a) => write!(f, "{}", a),
            AttributeValue::Vec3Float(_, _, _) => todo!(),
            AttributeValue::Vec3Int(_, _, _) => todo!(),
            AttributeValue::Vec3Uint(_, _, _) => todo!(),
            AttributeValue::Vec4Float(_, _, _, _) => todo!(),
            AttributeValue::Vec4Int(_, _, _, _) => todo!(),
            AttributeValue::Vec4Uint(_, _, _, _) => todo!(),
            AttributeValue::Bytes(a) => write!(f, "{:?}", a),
            AttributeValue::Any(a) => write!(f, "{:?}", a),
        }
    }
}

#[derive(Clone, Copy)]
#[allow(missing_docs)]
pub struct ArbitraryAttributeValue<'a> {
    pub value: &'a dyn std::any::Any,
    pub cmp: fn(&'a dyn std::any::Any, &'a dyn std::any::Any) -> bool,
}

impl PartialEq for ArbitraryAttributeValue<'_> {
    fn eq(&self, other: &Self) -> bool {
        (self.cmp)(self.value, other.value)
    }
}

impl std::fmt::Debug for ArbitraryAttributeValue<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ArbitraryAttributeValue").finish()
    }
}

#[cfg(feature = "serialize")]
impl<'a> serde::Serialize for ArbitraryAttributeValue<'a> {
    fn serialize<S>(&self, _serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        panic!("ArbitraryAttributeValue should not be serialized")
    }
}
#[cfg(feature = "serialize")]
impl<'de, 'a> serde::Deserialize<'de> for &'a ArbitraryAttributeValue<'a> {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        panic!("ArbitraryAttributeValue is not deserializable!")
    }
}
#[cfg(feature = "serialize")]
impl<'de, 'a> serde::Deserialize<'de> for ArbitraryAttributeValue<'a> {
    fn deserialize<D>(_deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        panic!("ArbitraryAttributeValue is not deserializable!")
    }
}

// todo
#[allow(missing_docs)]
impl<'a> AttributeValue<'a> {
    pub fn as_text(&self) -> Option<&'a str> {
        match self {
            AttributeValue::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_float32(&self) -> Option<f32> {
        match self {
            AttributeValue::Float32(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_float64(&self) -> Option<f64> {
        match self {
            AttributeValue::Float64(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_int32(&self) -> Option<i32> {
        match self {
            AttributeValue::Int32(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_int64(&self) -> Option<i64> {
        match self {
            AttributeValue::Int64(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_uint32(&self) -> Option<u32> {
        match self {
            AttributeValue::Uint32(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_uint64(&self) -> Option<u64> {
        match self {
            AttributeValue::Uint64(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            AttributeValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_vec3_float(&self) -> Option<(f32, f32, f32)> {
        match self {
            AttributeValue::Vec3Float(x, y, z) => Some((*x, *y, *z)),
            _ => None,
        }
    }

    pub fn as_vec3_int(&self) -> Option<(i32, i32, i32)> {
        match self {
            AttributeValue::Vec3Int(x, y, z) => Some((*x, *y, *z)),
            _ => None,
        }
    }

    pub fn as_vec3_uint(&self) -> Option<(u32, u32, u32)> {
        match self {
            AttributeValue::Vec3Uint(x, y, z) => Some((*x, *y, *z)),
            _ => None,
        }
    }

    pub fn as_vec4_float(&self) -> Option<(f32, f32, f32, f32)> {
        match self {
            AttributeValue::Vec4Float(x, y, z, w) => Some((*x, *y, *z, *w)),
            _ => None,
        }
    }

    pub fn as_vec4_int(&self) -> Option<(i32, i32, i32, i32)> {
        match self {
            AttributeValue::Vec4Int(x, y, z, w) => Some((*x, *y, *z, *w)),
            _ => None,
        }
    }

    pub fn as_vec4_uint(&self) -> Option<(u32, u32, u32, u32)> {
        match self {
            AttributeValue::Vec4Uint(x, y, z, w) => Some((*x, *y, *z, *w)),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            AttributeValue::Bytes(b) => Some(b),
            _ => None,
        }
    }

    pub fn as_any(&self) -> Option<&'a ArbitraryAttributeValue> {
        match self {
            AttributeValue::Any(a) => Some(a),
            _ => None,
        }
    }
}

/// A owned attribute value.
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(
    all(feature = "serialize"),
    derive(serde::Serialize, serde::Deserialize)
)]
#[allow(missing_docs)]
pub enum OwnedAttributeValue {
    Text(String),
    Float32(f32),
    Float64(f64),
    Int32(i32),
    Int64(i64),
    Uint32(u32),
    Uint64(u64),
    Bool(bool),

    Vec3Float(f32, f32, f32),
    Vec3Int(i32, i32, i32),
    Vec3Uint(u32, u32, u32),

    Vec4Float(f32, f32, f32, f32),
    Vec4Int(i32, i32, i32, i32),
    Vec4Uint(u32, u32, u32, u32),

    Bytes(Vec<u8>),
    // TODO: support other types
    // Any(ArbitraryAttributeValue<'a>),
}

impl PartialEq<AttributeValue<'_>> for OwnedAttributeValue {
    fn eq(&self, other: &AttributeValue<'_>) -> bool {
        match (self, other) {
            (Self::Text(l0), AttributeValue::Text(r0)) => l0 == r0,
            (Self::Float32(l0), AttributeValue::Float32(r0)) => l0 == r0,
            (Self::Float64(l0), AttributeValue::Float64(r0)) => l0 == r0,
            (Self::Int32(l0), AttributeValue::Int32(r0)) => l0 == r0,
            (Self::Int64(l0), AttributeValue::Int64(r0)) => l0 == r0,
            (Self::Uint32(l0), AttributeValue::Uint32(r0)) => l0 == r0,
            (Self::Uint64(l0), AttributeValue::Uint64(r0)) => l0 == r0,
            (Self::Bool(l0), AttributeValue::Bool(r0)) => l0 == r0,
            (Self::Vec3Float(l0, l1, l2), AttributeValue::Vec3Float(r0, r1, r2)) => {
                l0 == r0 && l1 == r1 && l2 == r2
            }
            (Self::Vec3Int(l0, l1, l2), AttributeValue::Vec3Int(r0, r1, r2)) => {
                l0 == r0 && l1 == r1 && l2 == r2
            }
            (Self::Vec3Uint(l0, l1, l2), AttributeValue::Vec3Uint(r0, r1, r2)) => {
                l0 == r0 && l1 == r1 && l2 == r2
            }
            (Self::Vec4Float(l0, l1, l2, l3), AttributeValue::Vec4Float(r0, r1, r2, r3)) => {
                l0 == r0 && l1 == r1 && l2 == r2 && l3 == r3
            }
            (Self::Vec4Int(l0, l1, l2, l3), AttributeValue::Vec4Int(r0, r1, r2, r3)) => {
                l0 == r0 && l1 == r1 && l2 == r2 && l3 == r3
            }
            (Self::Vec4Uint(l0, l1, l2, l3), AttributeValue::Vec4Uint(r0, r1, r2, r3)) => {
                l0 == r0 && l1 == r1 && l2 == r2 && l3 == r3
            }
            (Self::Bytes(l0), AttributeValue::Bytes(r0)) => l0 == r0,
            (_, _) => false,
        }
    }
}

impl<'a> From<AttributeValue<'a>> for OwnedAttributeValue {
    fn from(attr: AttributeValue<'a>) -> Self {
        match attr {
            AttributeValue::Text(t) => OwnedAttributeValue::Text(t.to_owned()),
            AttributeValue::Float32(f) => OwnedAttributeValue::Float32(f),
            AttributeValue::Float64(f) => OwnedAttributeValue::Float64(f),
            AttributeValue::Int32(i) => OwnedAttributeValue::Int32(i),
            AttributeValue::Int64(i) => OwnedAttributeValue::Int64(i),
            AttributeValue::Uint32(u) => OwnedAttributeValue::Uint32(u),
            AttributeValue::Uint64(u) => OwnedAttributeValue::Uint64(u),
            AttributeValue::Bool(b) => OwnedAttributeValue::Bool(b),
            AttributeValue::Vec3Float(f1, f2, f3) => OwnedAttributeValue::Vec3Float(f1, f2, f3),
            AttributeValue::Vec3Int(f1, f2, f3) => OwnedAttributeValue::Vec3Int(f1, f2, f3),
            AttributeValue::Vec3Uint(f1, f2, f3) => OwnedAttributeValue::Vec3Uint(f1, f2, f3),
            AttributeValue::Vec4Float(f1, f2, f3, f4) => {
                OwnedAttributeValue::Vec4Float(f1, f2, f3, f4)
            }
            AttributeValue::Vec4Int(f1, f2, f3, f4) => OwnedAttributeValue::Vec4Int(f1, f2, f3, f4),
            AttributeValue::Vec4Uint(f1, f2, f3, f4) => {
                OwnedAttributeValue::Vec4Uint(f1, f2, f3, f4)
            }
            AttributeValue::Bytes(b) => OwnedAttributeValue::Bytes(b.to_owned()),
            AttributeValue::Any(_) => todo!(),
        }
    }
}

// todo
#[allow(missing_docs)]
impl OwnedAttributeValue {
    pub fn as_text(&self) -> Option<&str> {
        match self {
            OwnedAttributeValue::Text(s) => Some(s),
            _ => None,
        }
    }

    pub fn as_float32(&self) -> Option<f32> {
        match self {
            OwnedAttributeValue::Float32(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_float64(&self) -> Option<f64> {
        match self {
            OwnedAttributeValue::Float64(f) => Some(*f),
            _ => None,
        }
    }

    pub fn as_int32(&self) -> Option<i32> {
        match self {
            OwnedAttributeValue::Int32(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_int64(&self) -> Option<i64> {
        match self {
            OwnedAttributeValue::Int64(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_uint32(&self) -> Option<u32> {
        match self {
            OwnedAttributeValue::Uint32(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_uint64(&self) -> Option<u64> {
        match self {
            OwnedAttributeValue::Uint64(i) => Some(*i),
            _ => None,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            OwnedAttributeValue::Bool(b) => Some(*b),
            _ => None,
        }
    }

    pub fn as_vec3_float(&self) -> Option<(f32, f32, f32)> {
        match self {
            OwnedAttributeValue::Vec3Float(x, y, z) => Some((*x, *y, *z)),
            _ => None,
        }
    }

    pub fn as_vec3_int(&self) -> Option<(i32, i32, i32)> {
        match self {
            OwnedAttributeValue::Vec3Int(x, y, z) => Some((*x, *y, *z)),
            _ => None,
        }
    }

    pub fn as_vec3_uint(&self) -> Option<(u32, u32, u32)> {
        match self {
            OwnedAttributeValue::Vec3Uint(x, y, z) => Some((*x, *y, *z)),
            _ => None,
        }
    }

    pub fn as_vec4_float(&self) -> Option<(f32, f32, f32, f32)> {
        match self {
            OwnedAttributeValue::Vec4Float(x, y, z, w) => Some((*x, *y, *z, *w)),
            _ => None,
        }
    }

    pub fn as_vec4_int(&self) -> Option<(i32, i32, i32, i32)> {
        match self {
            OwnedAttributeValue::Vec4Int(x, y, z, w) => Some((*x, *y, *z, *w)),
            _ => None,
        }
    }

    pub fn as_vec4_uint(&self) -> Option<(u32, u32, u32, u32)> {
        match self {
            OwnedAttributeValue::Vec4Uint(x, y, z, w) => Some((*x, *y, *z, *w)),
            _ => None,
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            OwnedAttributeValue::Bytes(b) => Some(b),
            _ => None,
        }
    }
}
