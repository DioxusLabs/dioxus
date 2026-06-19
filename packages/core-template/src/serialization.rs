#[cfg(feature = "serialize")]
pub fn deserialize_string_leaky<'de, D>(deserializer: D) -> Result<&'static str, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let deserialized = String::deserialize(deserializer)?;
    Ok(&*Box::leak(deserialized.into_boxed_str()))
}

#[cfg(feature = "serialize")]
pub fn deserialize_leaky<'de, T, D>(deserializer: D) -> Result<&'static [T], D::Error>
where
    T: serde::Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let deserialized = Box::<[T]>::deserialize(deserializer)?;
    Ok(&*Box::leak(deserialized))
}

#[cfg(feature = "serialize")]
pub fn deserialize_strings_leaky<'de, D>(
    deserializer: D,
) -> Result<&'static [&'static str], D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let deserialized = Vec::<String>::deserialize(deserializer)?;
    let strings: Vec<&'static str> = deserialized
        .into_iter()
        .map(|string| &*Box::leak(string.into_boxed_str()))
        .collect::<Vec<_>>();
    Ok(&*Box::leak(strings.into_boxed_slice()))
}

#[cfg(feature = "serialize")]
pub fn deserialize_option_leaky<'de, D>(deserializer: D) -> Result<Option<&'static str>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use serde::Deserialize;

    let deserialized = Option::<String>::deserialize(deserializer)?;
    Ok(deserialized.map(|deserialized| &*Box::leak(deserialized.into_boxed_str())))
}
