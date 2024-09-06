use crate::DioxusCrate;
use anyhow::Context;
use build::TargetArgs;
use once_cell::sync::Lazy;
use std::collections::HashMap;

use super::*;

/// Clean build artifacts.
#[derive(Clone, Debug, Parser)]
#[clap(name = "clean")]
pub struct Clean {}

static N_TO_IEC_UNIT_MAP: Lazy<HashMap<i32, &str>> = Lazy::new(|| {
    HashMap::<i32, &str>::from([
        (1, "K"),
        (2, "M"),
        (3, "G"),
        (4, "T"),
        (5, "P"),
        (6, "E"),
        (7, "Z"),
        (8, "Y"),
    ])
});

static IEC_UNIT_TO_N_MAP: Lazy<HashMap<&str, i32>> = Lazy::new(|| {
    HashMap::<&str, i32>::from([
        ("B", 0),
        ("KiB", 1),
        ("MiB", 2),
        ("GiB", 3),
        ("TiB", 4),
        ("PiB", 5),
        ("EiB", 6),
        ("ZiB", 7),
        ("YiB", 8),
    ])
});

impl Clean {
    pub fn clean(self) -> anyhow::Result<()> {
        // Tries to access Internet when offline.
        let dioxus_crate =
            DioxusCrate::new(&TargetArgs::default()).context("Failed to load Dioxus workspace")?;

        let output = Command::new("cargo")
            .arg("clean")
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()?;

        if !output.status.success() {
            return Err(anyhow::anyhow!("Cargo clean failed."));
        }

        let error = "Failed to get cargo clean output";
        let stderr = String::from_utf8(output.stderr).context(error)?;
        let summary_line = stderr.lines().last().context(error)?;
        dbg!(summary_line);
        let total = summary_line
            .split(' ')
            .rev()
            .nth(1)
            .context("Failed to parse cargo clean's output")?;

        let bytes1 = if total != "0" {
            let bytes = iec_size_to_bytes({
                let (a, b) =
                    total.split_at(total.rfind(|ch: char| ch.is_ascii_digit()).unwrap() + 1);
                &format!("{a} {b}")
            });
            eprintln!("Removed {bytes} bytes");
            bytes
        } else {
            0
        };

        let out_dir = &dioxus_crate.out_dir();
        dbg!(&out_dir);

        let bytes2 = if out_dir.is_dir() {
            eprintln!("out_dir exists");
            let size = fs_extra::dir::get_size(out_dir)
                .context("Failed to get size of the Dioxus `out_dir`");
            remove_dir_all(out_dir)?;
            let size = size?;
            eprintln!("Removed {} (Dioxus-specific)", bytes_to_iec_size(size));
            size
        } else {
            0
        };

        eprintln!("Removed {} total", bytes_to_iec_size(bytes1 + bytes2));

        Ok(())
    }
}

// Stolen from Typst.
/// Returns value with `n` digits after floating point where `n` is `precision`.
/// Standard rounding rules apply (if `n+1`th digit >= 5, round up).
///
/// If rounding the `value` will have no effect (e.g., it's infinite or
/// NaN), returns `value` unchanged.
///
/// # Examples
///
/// ```
/// # use typst_utils::format::round_with_precision;
/// let rounded = round_with_precision(-0.56553, 2);
/// assert_eq!(-0.57, rounded);
/// ```
pub fn round_with_precision(value: f64, precision: u8) -> f64 {
    // Don't attempt to round the float if that wouldn't have any effect. This
    // includes infinite or NaN values, as well as integer values
    // with a filled mantissa (which can't have a fractional part). Rounding
    // with a precision larger than the amount of digits that can be
    // effectively represented would also be a no-op.
    if value.is_infinite()
        || value.is_nan()
        || value.abs() >= (1_i64 << f64::MANTISSA_DIGITS) as f64
        || precision as u32 >= f64::DIGITS
    {
        return value;
    }
    let offset = 10_f64.powi(precision.into());
    assert!(
        (value * offset).is_finite(),
        "{value} * {offset} is not finite!"
    );
    (value * offset).round() / offset
}

fn bytes_to_iec_size(size: u64) -> String {
    if size < 1024 {
        format!("{size} B")
    } else {
        let n = (size.ilog2() / 10) as i32;
        format!(
            "{} {}iB",
            round_with_precision(size as f64 / 1024_f64.powi(n), 2),
            N_TO_IEC_UNIT_MAP.get(&n).expect("Size is too large")
        )
    }
}

/// Not needed if we can directly count the size of /target dir and if only **it** is
/// deleted via `cargo crate`. Otherwise used for parsing output of `cargo clean`
/// (not accurate) and then sum the 2 size values.
fn iec_size_to_bytes(size: &str) -> u64 {
    let (value, unit) = size
        .split_once(' ')
        .unwrap_or_else(|| panic!("tried splitting |{size}| on space"));
    dbg!(value, unit);
    let n = IEC_UNIT_TO_N_MAP
        .get(unit)
        .expect("Size is too large or invalid unit");
    (value.parse::<f64>().unwrap() * 1024_f64.powi(*n)).round() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bytes_to_iec_size() {
        assert_eq!("0 B", &bytes_to_iec_size(0));
        assert_eq!("1 B", &bytes_to_iec_size(1));
        assert_eq!("1023 B", &bytes_to_iec_size(1023));
        assert_eq!("1 KiB", &bytes_to_iec_size(1024));
        assert_eq!("1.46 KiB", &bytes_to_iec_size(1500));
        assert_eq!("1 MiB", &bytes_to_iec_size(1024 * 1024));
        assert_eq!("1 GiB", &bytes_to_iec_size(1024 * 1024 * 1024));
        assert_eq!("1 TiB", &bytes_to_iec_size(1024 * 1024 * 1024 * 1024));
        static GIB: u64 = 1024 * 1024 * 1024 * 1024;
        assert_eq!("1 PiB", &bytes_to_iec_size(GIB * 1024));
    }

    #[test]
    fn test_iec_size_to_bytes() {
        assert_eq!(0, iec_size_to_bytes("0 B"));
        assert_eq!(1, iec_size_to_bytes("1 B"));
        assert_eq!(1023, iec_size_to_bytes("1023 B"));
        assert_eq!(1024, iec_size_to_bytes("1 KiB"));
        assert_eq!(1500, iec_size_to_bytes("1.46 KiB"));
        assert_eq!(1024 * 1024, iec_size_to_bytes("1 MiB"));
        assert_eq!(1024 * 1024 * 1024, iec_size_to_bytes("1 GiB"));
        assert_eq!(1024 * 1024 * 1024 * 1024, iec_size_to_bytes("1 TiB"));
        static GIB: u64 = 1024 * 1024 * 1024 * 1024;
        assert_eq!(GIB * 1024, iec_size_to_bytes("1 PiB"));
    }
}
