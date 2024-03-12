use serde::Serialize;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

#[allow(unused)]
pub(crate) fn serde_to_writable<T: Serialize>(
    value: &T,
    write_to: &mut impl std::io::Write,
) -> Result<(), ciborium::ser::Error<std::io::Error>> {
    let mut serialized = Vec::new();
    ciborium::into_writer(value, &mut serialized)?;
    write_to.write_all(STANDARD.encode(serialized).as_bytes())?;
    Ok(())
}

#[cfg(feature = "server")]
/// Encode data into a element. This is inteded to be used in the server to send data to the client.
pub(crate) fn encode_props_in_element<T: Serialize>(
    data: &T,
    write_to: &mut impl std::io::Write,
) -> Result<(), ciborium::ser::Error<std::io::Error>> {
    write_to.write_all(
        r#"<meta hidden="true" id="dioxus-storage-props" data-serialized=""#.as_bytes(),
    )?;
    serde_to_writable(data, write_to)?;
    Ok(write_to.write_all(r#"" />"#.as_bytes())?)
}

#[cfg(feature = "server")]
/// Encode data into a element. This is inteded to be used in the server to send data to the client.
pub(crate) fn encode_in_element(
    data: &super::HTMLData,
    write_to: &mut impl std::io::Write,
) -> Result<(), ciborium::ser::Error<std::io::Error>> {
    write_to.write_all(
        r#"<meta hidden="true" id="dioxus-storage-data" data-serialized=""#.as_bytes(),
    )?;
    serde_to_writable(&data, write_to)?;
    Ok(write_to.write_all(r#"" />"#.as_bytes())?)
}
