use serde::Serialize;

use super::u16_to_char;

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
    for array in compressed.chunks(2) {
        let w = if array.len() == 2 {
            [array[0], array[1]]
        } else {
            [array[0], 0]
        };
        write_to.write_char(u16_to_char((w[0] as u16) << 8 | (w[1] as u16)))?;
    }
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
