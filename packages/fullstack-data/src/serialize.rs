use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use serde::Serialize;

#[allow(unused)]
pub(crate) fn serde_to_writable<T: Serialize>(
    value: &T,
    write_to: &mut impl std::fmt::Write,
) -> Result<(), ciborium::ser::Error<std::fmt::Error>> {
    let mut serialized = Vec::new();
    ciborium::into_writer(value, &mut serialized).unwrap();
    write_to.write_str(STANDARD.encode(serialized).as_str())?;
    Ok(())
}
