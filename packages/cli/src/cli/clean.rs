use crate::DioxusCrate;
use anyhow::Context;
use build::TargetArgs;
use std::{env, path::Path};
use walkdir::WalkDir;

use super::*; // ?

/// Clean build artifacts.
#[derive(Clone, Debug, Parser)]
#[clap(name = "clean")]
pub struct Clean {}

impl Clean {
    pub fn clean(self) -> anyhow::Result<()> {
        // Tries to access Internet when offline. Creates a new file every time.
        let dioxus_crate =
            DioxusCrate::new(&TargetArgs::default()).context("Failed to load Dioxus workspace")?;

        // TODO: remove dbg and unwraps

        let target_dir = &dioxus_crate.workspace_dir().join(
            env::var_os("CARGO_BUILD_TARGET_DIR")
                .or_else(|| env::var_os("CARGO_TARGET_DIR"))
                .unwrap_or("target".into()),
        );

        dbg!(target_dir);

        let (target_dir_file_count, target_dir_size) = if target_dir.is_dir() {
            let files = count_files(target_dir);
            let size = fs_extra::dir::get_size(target_dir).unwrap();
            remove_dir_all(target_dir)?;
            (files, size)
        } else {
            (0, 0)
        };
        dbg!(target_dir_file_count, target_dir_size);

        let out_dir = &dioxus_crate.out_dir();
        dbg!(&out_dir);

        let (out_dir_file_count, out_dir_size) = if out_dir.is_dir() {
            let files = count_files(out_dir);
            let size = fs_extra::dir::get_size(out_dir).unwrap();
            remove_dir_all(out_dir)?;
            (files, size)
        } else {
            (0, 0)
        };
        dbg!(out_dir_file_count, out_dir_size);

        eprintln!(
            "Removed {} files, {} total",
            target_dir_file_count + out_dir_file_count,
            bytes_to_iec_size(target_dir_size + out_dir_size)
        );

        Ok(())
    }
}

fn count_files<P>(root: P) -> usize
where
    P: AsRef<Path>,
{
    WalkDir::new(root)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().is_file())
        .count()
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
    let n = (size.ilog2() / 10) as i32;
    let value = round_with_precision(size as f64 / 1024_f64.powi(n), 2);
    let unit = match n {
        0 => "B",
        1 => "KiB",
        2 => "MiB",
        3 => "GiB",
        4 => "TiB",
        5 => "PiB",
        6 => "EiB",
        7 => "ZiB",
        8 => "YiB",
        _ => unreachable!("Size is too large"),
    };
    format!("{value} {unit}")
}

#[cfg(test)]
mod tests {
    use super::*;

    // TODO: add test for the clean subcommand.

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
}
