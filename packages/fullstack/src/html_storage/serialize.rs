use serde::Serialize;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

#[allow(unused)]
pub(crate) fn serde_to_writable<T: Serialize>(
    value: &T,
    write_to: &mut impl std::io::Write,
) -> std::io::Result<()> {
    let serialized = postcard::to_allocvec(value).unwrap();
    write_to.write_all(STANDARD.encode(serialized).as_bytes())?;
    Ok(())
}

#[cfg(feature = "ssr")]
/// Encode data into a element. This is inteded to be used in the server to send data to the client.
pub(crate) fn encode_props_in_element<T: Serialize>(
    data: &T,
    write_to: &mut impl std::io::Write,
) -> std::io::Result<()> {
    write_to.write_all(
        r#"<meta hidden="true" id="dioxus-storage-props" data-serialized=""#.as_bytes(),
    )?;
    serde_to_writable(data, write_to)?;
    write_to.write_all(r#"" />"#.as_bytes())
}

#[cfg(feature = "ssr")]
/// Encode data into a element. This is inteded to be used in the server to send data to the client.
pub(crate) fn encode_in_element(
    data: &super::HTMLData,
    write_to: &mut impl std::io::Write,
) -> std::io::Result<()> {
    write_to.write_all(
        r#"<meta hidden="true" id="dioxus-storage-data" data-serialized=""#.as_bytes(),
    )?;
    serde_to_writable(&data, write_to)?;
    write_to.write_all(r#"" />"#.as_bytes())
}
