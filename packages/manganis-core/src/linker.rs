/// Information about the manganis link section for a given platform
#[derive(Debug, Clone, Copy)]
pub struct LinkSection {
    /// The link section we pass to the static
    pub link_section: &'static str,
    /// The name of the section we find in the binary
    pub name: &'static str,
}

impl LinkSection {
    /// The list of link sections for all supported platforms
    pub const ALL: &'static [&'static LinkSection] =
        &[Self::WASM, Self::MACOS, Self::WINDOWS, Self::ILLUMOS];

    /// Returns the link section used in linux, android, fuchsia, psp, freebsd, and wasm32
    pub const WASM: &'static LinkSection = &LinkSection {
        link_section: "manganis",
        name: "manganis",
    };

    /// Returns the link section used in macOS, iOS, tvOS
    pub const MACOS: &'static LinkSection = &LinkSection {
        link_section: "__DATA,manganis,regular,no_dead_strip",
        name: "manganis",
    };

    /// Returns the link section used in windows
    pub const WINDOWS: &'static LinkSection = &LinkSection {
        link_section: "mg",
        name: "mg",
    };

    /// Returns the link section used in illumos
    pub const ILLUMOS: &'static LinkSection = &LinkSection {
        link_section: "set_manganis",
        name: "set_manganis",
    };

    /// The link section used on the current platform
    pub const CURRENT: &'static LinkSection = {
        #[cfg(any(
            target_os = "none",
            target_os = "linux",
            target_os = "android",
            target_os = "fuchsia",
            target_os = "psp",
            target_os = "freebsd",
            target_arch = "wasm32"
        ))]
        {
            Self::WASM
        }

        #[cfg(any(target_os = "macos", target_os = "ios", target_os = "tvos"))]
        {
            Self::MACOS
        }

        #[cfg(target_os = "windows")]
        {
            Self::WINDOWS
        }

        #[cfg(target_os = "illumos")]
        {
            Self::ILLUMOS
        }
    };
}
