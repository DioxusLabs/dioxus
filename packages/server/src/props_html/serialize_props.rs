use serde::Serialize;

use base64::engine::general_purpose::STANDARD;
use base64::Engine;

#[allow(unused)]
pub(crate) fn serde_to_writable<T: Serialize>(
    value: &T,
    mut write_to: impl std::fmt::Write,
) -> std::fmt::Result {
    let serialized = postcard::to_allocvec(value).unwrap();
    let compressed = yazi::compress(
        &serialized,
        yazi::Format::Zlib,
        yazi::CompressionLevel::BestSize,
    )
    .unwrap();
    write_to.write_str(&STANDARD.encode(compressed));
    Ok(())
}

#[cfg(feature = "ssr")]
/// Encode data into a element. This is inteded to be used in the server to send data to the client.
pub(crate) fn encode_in_element<T: Serialize>(
    data: T,
    mut write_to: impl std::fmt::Write,
) -> std::fmt::Result {
    write_to.write_str(r#"<meta hidden="true" id="dioxus-storage" data-serialized=""#)?;
    serde_to_writable(&data, &mut write_to)?;
    write_to.write_str(r#"" />"#)
}
