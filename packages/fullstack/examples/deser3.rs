use http::request::Parts;
use serde::de::DeserializeSeed;

fn main() {}

struct Deser<T> {
    _phantom: std::marker::PhantomData<T>,
}

struct Extractor<'a> {
    parts: &'a mut Parts,
}

impl<'de, 'a> DeserializeSeed<'de> for Extractor<'a> {
    type Value = ();

    fn deserialize<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct Visitor<'a> {
            parts: &'a mut Parts,
        }

        impl<'de, 'a> serde::de::Visitor<'de> for Visitor<'a> {
            type Value = ();

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("an extractor")
            }

            fn visit_map<A>(self, map: A) -> Result<Self::Value, A::Error>
            where
                A: serde::de::MapAccess<'de>,
            {
                todo!()
            }
        }

        todo!()
    }
}
