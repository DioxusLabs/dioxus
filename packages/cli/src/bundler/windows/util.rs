//! Windows bundler utility functions.
//!
//! Constants and helpers shared by NSIS and MSI bundling.

/// Output folder name for NSIS installers within the bundle directory.
pub(crate) const NSIS_OUTPUT_FOLDER_NAME: &str = "nsis";

/// Convert a BundleContext's Arch to a Windows architecture string
/// suitable for installer file names and WebView2 downloads.
pub(crate) fn arch_to_windows_string(arch: &crate::bundler::context::Arch) -> &'static str {
    use crate::bundler::context::Arch;
    match arch {
        Arch::X86_64 => "x64",
        Arch::X86 => "x86",
        Arch::AArch64 => "arm64",
        _ => "x64", // Default to x64 for unsupported architectures
    }
}
