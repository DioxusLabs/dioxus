#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(clippy::all)]
//! Modified version of https://github.com/BenjaminRi/winresource or https://github.com/tauri-apps/winres
//!
//! Rust Windows resource helper
//!
//! This crate implements a simple generator for Windows resource (.rc) files
//! for use with either Microsoft `rc.exe` resource compiler or with GNU `windres.exe`
//!
//! The [`WindowsResource::compile()`] method is intended to be used from a build script and
//! needs environment variables from cargo to be set. It not only compiles the resource
//! but directs cargo to link the resource compiler's output.
//!
//! # Example
//!
//! ```rust
//! # extern crate winres;
//! # use std::io;
//! # fn test_main() -> io::Result<()> {
//! if cfg!(target_os = "windows") {
//!     let mut res = winres::WindowsResource::new();
//!     res.set_icon("test.ico")
//! #      .set_output_directory(".")
//!        .set("InternalName", "TEST.EXE")
//!        // manually set version 1.0.0.0
//!        .set_version_info(winres::VersionInfo::PRODUCTVERSION, 0x0001000000000000);
//!     res.compile()?;
//! }
//! # Ok(())
//! # }
//! ```
//!
//! # Defaults
//!
//! We try to guess some sensible default values from Cargo's build time environment variables
//! This is described in [`WindowsResource::new()`]. Furthermore we have to know where to find the
//! resource compiler for the MSVC Toolkit. This can be done by looking up a registry key but
//! for MinGW this has to be done manually.
//!
//! The following paths are the hardcoded defaults:
//! MSVC the last registry key at
//! `HKLM\SOFTWARE\Microsoft\Windows Kits\Installed Roots`, for MinGW we try our luck by simply
//! using the `%PATH%` environment variable.
//!
//! Note that the toolkit bitness as to match the one from the current Rust compiler. If you are
//! using Rust GNU 64-bit you have to use MinGW64. For MSVC this is simpler as (recent) Windows
//! SDK always installs both versions on a 64-bit system.
//!
//! [`WindowsResource::compile()`]: struct.WindowsResource.html#method.compile
//! [`WindowsResource::new()`]: struct.WindowsResource.html#method.new
#![allow(dead_code)]

use krates::semver::Version;

use crate::BuildRequest;
use std::collections::HashMap;
use std::env;
use std::fs;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process;

/// https://learn.microsoft.com/en-us/windows/win32/menurc/about-icons
/// use to_str()
#[allow(clippy::upper_case_acronyms)]
pub enum IDI {
    APPLICATION = 32512,
    ERROR = 32513,
    QUESTION = 32514,
    WARNING = 32515,
    INFORMATION = 32516,
    WINLOGO = 32517,
    SHIELD = 32518,
}

impl IDI {
    pub fn as_str(&self) -> &'static str {
        match self {
            IDI::APPLICATION => "32512",
            IDI::ERROR => "32513",
            IDI::QUESTION => "32514",
            IDI::WARNING => "32515",
            IDI::INFORMATION => "32516",
            IDI::WINLOGO => "32517",
            IDI::SHIELD => "32518",
        }
    }
}

/// Version info field names
#[allow(clippy::upper_case_acronyms)]
#[derive(PartialEq, Eq, Hash, Debug)]
pub enum VersionInfo {
    /// The version value consists of four 16 bit words, e.g.,
    /// `MAJOR << 48 | MINOR << 32 | PATCH << 16 | RELEASE`
    FILEVERSION,
    /// The version value consists of four 16 bit words, e.g.,
    /// `MAJOR << 48 | MINOR << 32 | PATCH << 16 | RELEASE`
    PRODUCTVERSION,
    /// Should be Windows NT Win32, with value `0x40004`
    FILEOS,
    /// The value (for a rust compiler output) should be
    /// 1 for a EXE and 2 for a DLL
    FILETYPE,
    /// Only for Windows drivers
    FILESUBTYPE,
    /// Bit mask for FILEFLAGS
    FILEFLAGSMASK,
    /// Only the bits set in FILEFLAGSMASK are read
    FILEFLAGS,
}

impl VersionInfo {
    /// Creates u64 version from string
    pub fn version_from_str(version: &str) -> u64 {
        let parts: Vec<&str> = version.split('.').collect();
        if parts.len() > 4 {
            tracing::warn!("Version number had more than 4 parts. Ignoring the rest.");
        }
        let mut segments = [0u16; 4];
        for (i, part) in parts.iter().take(4).enumerate() {
            match part.parse::<u16>() {
                Ok(value) => segments[i] = value,
                Err(e) => {
                    tracing::warn!(
                        "Could not parse segment {} '{}' as u16: {}. Defaulting to 0.",
                        i,
                        part,
                        e
                    );
                }
            }
        }

        ((segments[0] as u64) << 48)
            | ((segments[1] as u64) << 32)
            | ((segments[2] as u64) << 16)
            | (segments[3] as u64)
    }

    pub fn version_from_krate(v: &Version) -> u64 {
        (v.major << 48) | (v.minor << 32) | (v.patch << 16)
    }
}

/// Windows uses `32512` as the default icon ID.
const DEFAULT_ICON_ID: &str = "32512";
#[derive(Debug)]
struct Icon {
    path: String,
    name_id: String,
}

impl Icon {
    fn from_pathbuf(path: &Path, id: &str) -> Self {
        Icon {
            path: path
                .canonicalize()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or(path.to_string_lossy().to_string()),
            name_id: id.into(),
        }
    }

    fn from_str(path: &str, id: &str) -> Self {
        Icon {
            path: PathBuf::from(path)
                .canonicalize()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or(path.to_string()),
            name_id: id.into(),
        }
    }
}
#[derive(Debug, Default)]
pub struct WindowsResourceLinker {
    pub lib: String,
    pub path: String,
    pub files: Vec<String>,
    pub default_icon: Option<String>,
}

#[derive(Debug)]
pub struct WindowsResource {
    pub linker: WindowsResourceLinker,
    toolkit_path: PathBuf,
    properties: HashMap<String, String>,
    version_info: HashMap<VersionInfo, u64>,
    rc_file: Option<String>,
    icons: Vec<Icon>,
    language: u16,
    manifest: Option<String>,
    manifest_file: Option<String>,
    output_directory: String,
    windres_path: String,
    ar_path: String,
    add_toolkit_include: bool,
    append_rc_content: String,
    assets_path: Option<String>,
    link: bool,
}


impl WindowsResource {
    /// Create a new resource with version info struct
    ///
    ///
    /// We initialize the resource file with values provided by cargo
    ///
    /// | Field                | Cargo / Values               |
    /// |----------------------|------------------------------|
    /// | `"FileVersion"`      | `package.version`            |
    /// | `"ProductVersion"`   | `package.version`            |
    /// | `"ProductName"`      | `package.name`               |
    /// | `"FileDescription"`  | `package.description`        |
    ///
    /// Furthermore if a section `package.metadata.winres` exists
    /// in `Cargo.toml` it will be parsed. Values in this section take precedence
    /// over the values provided natively by cargo. Only the string table
    /// of the version struct can be set this way.
    /// Additionally, the language field is set to neutral (i.e. `0`)
    /// and no icon is set. These settings have to be done programmatically.
    ///
    /// `Cargo.toml` files have to be written in UTF-8, so we support all valid UTF-8 strings
    /// provided.
    ///
    /// ```,toml
    /// #Cargo.toml
    /// [package.metadata.winres]
    /// OriginalFilename = "testing.exe"
    /// FileDescription = "⛄❤☕"
    /// LegalCopyright = "Copyright © 2016"
    /// ```
    ///
    /// The version info struct is set to some values
    /// sensible for creating an executable file.
    ///
    /// | Property             | Cargo / Values               |
    /// |----------------------|------------------------------|
    /// | `FILEVERSION`        | `package.version`            |
    /// | `PRODUCTVERSION`     | `package.version`            |
    /// | `FILEOS`             | `VOS_NT_WINDOWS32 (0x40004)` |
    /// | `FILETYPE`           | `VFT_APP (0x1)`              |
    /// | `FILESUBTYPE`        | `VFT2_UNKNOWN (0x0)`         |
    /// | `FILEFLAGSMASK`      | `VS_FFI_FILEFLAGSMASK (0x3F)`|
    /// | `FILEFLAGS`          | `0x0`                        |
    ///
    pub fn new() -> Self {
        let props: HashMap<String, String> = HashMap::new();
        let mut ver: HashMap<VersionInfo, u64> = HashMap::new();

        ver.insert(VersionInfo::FILEVERSION, 0);
        ver.insert(VersionInfo::PRODUCTVERSION, 0);
        ver.insert(VersionInfo::FILEOS, 0x00040004);
        ver.insert(VersionInfo::FILETYPE, 1);
        ver.insert(VersionInfo::FILESUBTYPE, 0);
        ver.insert(VersionInfo::FILEFLAGSMASK, 0x3F);
        ver.insert(VersionInfo::FILEFLAGS, 0);

        let sdk = if cfg!(target_env = "msvc") {
            match get_sdk() {
                Ok(mut v) => v.pop().unwrap(),
                Err(_) => PathBuf::new(),
            }
        } else if cfg!(windows) {
            PathBuf::from("\\")
        } else {
            PathBuf::from("/")
        };

        let prefix = if let Ok(cross) = env::var("CROSS_COMPILE") {
            cross
        } else if cfg!(not(target_env = "msvc"))
            && env::var_os("HOST")
                .zip(env::var_os("TARGET"))
                .map_or(false, |(h, t)| h != t)
        {
            match env::var("TARGET").unwrap().as_str() {
                "x86_64-pc-windows-gnu" => "x86_64-w64-mingw32-",
                "i686-pc-windows-gnu" => "i686-w64-mingw32-",
                "i586-pc-windows-gnu" => "i586-w64-mingw32-",
                // MinGW supports ARM64 only with an LLVM-based toolchain
                // (x86 users might also be using LLVM, but we can't tell that from the Rust target...)
                "aarch64-pc-windows-gnu" => "llvm-",
                // *-gnullvm targets by definition use LLVM-based toolchains
                "x86_64-pc-windows-gnullvm"
                | "i686-pc-windows-gnullvm"
                | "aarch64-pc-windows-gnullvm" => "llvm-",
                // fail safe
                _ => {
                    println!(
                        "cargo:warning=unknown Windows target used for cross-compilation; \
                              invoking unprefixed windres"
                    );
                    ""
                }
            }
            .into()
        } else {
            "".into()
        };

        let windres_path = if let Ok(windres) = env::var("WINDRES") {
            windres
        } else {
            format!("{}windres", prefix)
        };
        let ar_path = if let Ok(ar) = env::var("AR") {
            ar
        } else {
            format!("{}ar", prefix)
        };

        WindowsResource {
            toolkit_path: sdk,
            properties: props,
            version_info: ver,
            rc_file: None,
            icons: Vec::new(),
            language: 0,
            manifest: None,
            manifest_file: None,
            output_directory: ".".to_string(),
            windres_path,
            ar_path,
            add_toolkit_include: false,
            append_rc_content: String::new(),
            assets_path: None,
            link: true,
            linker: WindowsResourceLinker::default(),
        }
    }


    /// Set string properties of the version info struct.
    ///
    /// Possible field names are:
    ///
    ///  - `"FileVersion"`
    ///  - `"FileDescription"`
    ///  - `"ProductVersion"`
    ///  - `"ProductName"`
    ///  - `"OriginalFilename"`
    ///  - `"LegalCopyright"`
    ///  - `"LegalTrademark"`
    ///  - `"CompanyName"`
    ///  - `"Comments"`
    ///  - `"InternalName"`
    ///
    /// Additionally there exists
    /// `"PrivateBuild"`, `"SpecialBuild"`
    /// which should only be set, when the `FILEFLAGS` property is set to
    /// `VS_FF_PRIVATEBUILD(0x08)` or `VS_FF_SPECIALBUILD(0x20)`
    ///
    /// It is possible to use arbitrary field names but Windows Explorer and other
    /// tools might not show them.
    pub fn set(&mut self, name: &str, value: &str) -> &mut Self {
        self.properties.insert(name.to_string(), value.to_string());
        self
    }

    /// Set the correct path for the toolkit.
    ///
    /// For the GNU toolkit this has to be the path where MinGW
    /// put `windres.exe` and `ar.exe`. This could be something like:
    /// `C:\Program Files\mingw-w64\x86_64-5.3.0-win32-seh-rt_v4-rev0\mingw64\bin`
    ///
    /// For MSVC the Windows SDK has to be installed. It comes with the resource compiler
    /// `rc.exe`. This should be set to the root directory of the Windows SDK, e.g.,
    /// `C:\Program Files (x86)\Windows Kits\10`
    /// or, if multiple 10 versions are installed,
    /// set it directly to the correct bin directory
    /// `C:\Program Files (x86)\Windows Kits\10\bin\10.0.14393.0\x64`
    ///
    /// If it is left unset, it will look up a path in the registry,
    /// i.e. `HKLM\SOFTWARE\Microsoft\Windows Kits\Installed Roots`
    pub fn set_toolkit_path(&mut self, path: &str) -> &mut Self {
        self.toolkit_path = PathBuf::from(path);
        self
    }

    /// Set the user interface language of the file
    ///
    /// # Example
    ///
    /// ```
    /// extern crate winapi;
    /// extern crate winres;
    /// # use std::io;
    /// fn main() {
    ///   if cfg!(target_os = "windows") {
    ///     let mut res = winres::WindowsResource::new();
    /// #   res.set_output_directory(".");
    ///     res.set_language(winapi::um::winnt::MAKELANGID(
    ///         winapi::um::winnt::LANG_ENGLISH,
    ///         winapi::um::winnt::SUBLANG_ENGLISH_US
    ///     ));
    ///     res.compile().unwrap();
    ///   }
    /// }
    /// ```
    /// For possible values look at the `winapi::um::winnt` constants, specifically those
    /// starting with `LANG_` and `SUBLANG_`.
    ///
    /// [`MAKELANGID`]: https://docs.rs/winapi/0.3/x86_64-pc-windows-msvc/winapi/um/winnt/fn.MAKELANGID.html
    /// [`winapi::um::winnt`]: https://docs.rs/winapi/0.3/x86_64-pc-windows-msvc/winapi/um/winnt/index.html#constants
    ///
    /// # Table
    /// Sometimes it is just simpler to specify the numeric constant directly
    /// (That is what most `.rc` files do).
    /// For possible values take a look at the MSDN page for resource files;
    /// we only listed some values here.
    ///
    /// | Language            | Value    |
    /// |---------------------|----------|
    /// | Neutral             | `0x0000` |
    /// | English             | `0x0009` |
    /// | English (US)        | `0x0409` |
    /// | English (GB)        | `0x0809` |
    /// | German              | `0x0407` |
    /// | German (AT)         | `0x0c07` |
    /// | French              | `0x000c` |
    /// | French (FR)         | `0x040c` |
    /// | Catalan             | `0x0003` |
    /// | Basque              | `0x042d` |
    /// | Breton              | `0x007e` |
    /// | Scottish Gaelic     | `0x0091` |
    /// | Romansch            | `0x0017` |
    pub fn set_language(&mut self, language: u16) -> &mut Self {
        self.language = language;
        self
    }

    /// Add an icon with nameID `1`.
    ///
    /// This icon need to be in `ico` format. The filename can be absolute
    /// or relative to the projects root.
    ///
    /// Equivalent ```to set_icon_with_id(path, idi_application)```. [`IDI::APPLICATION`].as_str()
    pub fn set_icon(&mut self, path: &str) -> &mut Self {
        self.set_icon_with_id(path, IDI::APPLICATION.as_str())
    }

    /// Add an icon with the specified name ID.
    ///
    /// This icon need to be in `ico` format. The path can be absolute or
    /// relative to the projects root.
    ///
    /// ## Name ID and Icon Loading
    ///
    /// The name ID can be (the string representation of) a 16-bit unsigned
    /// integer, or some other string.
    ///
    /// You should not add multiple icons with the same name ID. It will result
    /// in a build failure.
    ///
    /// When the name ID is an integer, the icon can be loaded at runtime with
    ///
    /// ```ignore
    /// LoadIconW(h_instance, MAKEINTRESOURCEW(name_id_as_integer))
    /// ```
    ///
    /// Otherwise, it can be loaded with
    ///
    /// ```ignore
    /// LoadIconW(h_instance, name_id_as_wide_c_str_as_ptr)
    /// ```
    ///
    /// Where `h_instance` is the module handle of the current executable
    /// ([`GetModuleHandleW`](https://docs.rs/winapi/0.3.8/winapi/um/libloaderapi/fn.GetModuleHandleW.html)`(null())`),
    /// [`LoadIconW`](https://docs.rs/winapi/0.3.8/winapi/um/winuser/fn.LoadIconW.html)
    /// and
    /// [`MAKEINTRESOURCEW`](https://docs.rs/winapi/0.3.8/winapi/um/winuser/fn.MAKEINTRESOURCEW.html)
    /// are defined in winapi.
    ///
    /// ## Multiple Icons, Which One is Application Icon?
    ///
    /// When you have multiple icons, it's a bit complicated which one will be
    /// chosen as the application icon:
    /// <https://docs.microsoft.com/en-us/previous-versions/ms997538(v=msdn.10)?redirectedfrom=MSDN#choosing-an-icon>.
    ///
    /// To keep things simple, we recommend you use only 16-bit unsigned integer
    /// name IDs, and add the application icon first with the lowest id:
    ///
    /// ```nocheck
    /// res.set_icon("icon.ico") // This is application icon.
    ///    .set_icon_with_id("icon2.icon", "2")
    ///    .set_icon_with_id("icon3.icon", "3")
    ///    // ...
    /// ```
    /// see [`IDI`]` for special icons ids
    pub fn set_icon_with_id(&mut self, path: &str, name_id: &str) -> &mut Self {
        self.icons.push(Icon {
            path: path.into(),
            name_id: name_id.into(),
        });
        self
    }

    /// Set a version info struct property
    /// Currently we only support numeric values; you have to look them up.
    pub fn set_version_info(&mut self, field: VersionInfo, value: u64) -> &mut Self {
        self.version_info.insert(field, value);
        self
    }

    /// Set the embedded manifest file
    ///
    /// # Example
    ///
    /// The following manifest will brand the exe as requesting administrator privileges.
    /// Thus, everytime it is executed, a Windows UAC dialog will appear.
    ///
    /// ```rust
    /// let mut res = winres::WindowsResource::new();
    /// res.set_manifest(r#"
    /// <assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
    /// <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    ///     <security>
    ///         <requestedPrivileges>
    ///             <requestedExecutionLevel le vel="requireAdministrator" uiAccess="false" />
    ///         </requestedPrivileges>
    ///     </security>
    /// </trustInfo>
    /// </assembly>
    /// "#);
    /// ```
    pub fn set_manifest(&mut self, manifest: &str) -> &mut Self {
        self.manifest_file = None;
        self.manifest = Some(manifest.to_string());
        self
    }

    /// Some as [`set_manifest()`] but a filename can be provided and
    /// file is included by the resource compiler itself.
    /// This method works the same way as [`set_icon()`]
    ///
    /// [`set_manifest()`]: #method.set_manifest
    /// [`set_icon()`]: #method.set_icon
    pub fn set_manifest_file(&mut self, file: &str) -> &mut Self {
        self.manifest_file = Some(file.to_string());
        self.manifest = None;
        self
    }

    /// Set the path to the windres executable.
    pub fn set_windres_path(&mut self, path: &str) -> &mut Self {
        self.windres_path = path.to_string();
        self
    }

    /// Set the path to the ar executable.
    pub fn set_ar_path(&mut self, path: &str) -> &mut Self {
        self.ar_path = path.to_string();
        self
    }

    /// Set the path to the ar executable.
    pub fn add_toolkit_include(&mut self, add: bool) -> &mut Self {
        self.add_toolkit_include = add;
        self
    }

    /// Set cargo manifest dir, if None defaults to env variable
    pub fn set_assets_path(&mut self, path: Option<String>) -> &mut Self {
        self.assets_path = path;
        self
    }

    /// Write a resource file with the set values
    pub fn write_resource_file<P: AsRef<Path>>(&self, path: P) -> io::Result<()> {
        let mut f = fs::File::create(path)?;

        // use UTF8 as an encoding
        // this makes it easier since in rust all string are UTF8
        writeln!(f, "#pragma code_page(65001)")?;
        writeln!(f, "1 VERSIONINFO")?;
        for (k, v) in self.version_info.iter() {
            match *k {
                VersionInfo::FILEVERSION | VersionInfo::PRODUCTVERSION => writeln!(
                    f,
                    "{:?} {}, {}, {}, {}",
                    k,
                    (*v >> 48) as u16,
                    (*v >> 32) as u16,
                    (*v >> 16) as u16,
                    *v as u16
                )?,
                _ => writeln!(f, "{:?} {:#x}", k, v)?,
            };
        }
        writeln!(f, "{{\nBLOCK \"StringFileInfo\"")?;
        writeln!(f, "{{\nBLOCK \"{:04x}04b0\"\n{{", self.language)?;
        for (k, v) in self.properties.iter() {
            if !v.is_empty() {
                writeln!(
                    f,
                    "VALUE \"{}\", \"{}\"",
                    escape_string(k),
                    escape_string(v)
                )?;
            }
        }
        writeln!(f, "}}\n}}")?;

        writeln!(f, "BLOCK \"VarFileInfo\" {{")?;
        writeln!(f, "VALUE \"Translation\", {:#x}, 0x04b0", self.language)?;
        writeln!(f, "}}\n}}")?;
        for icon in &self.icons {
            writeln!(
                f,
                "{} ICON \"{}\"",
                escape_string(&icon.name_id),
                escape_string(&icon.path)
            )?;
        }
        if let Some(e) = self.version_info.get(&VersionInfo::FILETYPE) {
            if let Some(manf) = self.manifest.as_ref() {
                writeln!(f, "{} 24", e)?;
                writeln!(f, "{{")?;
                for line in manf.lines() {
                    writeln!(f, "\" {} \"", escape_string(line.trim()))?;
                }
                writeln!(f, "}}")?;
            } else if let Some(manf) = self.manifest_file.as_ref() {
                writeln!(f, "{} 24 \"{}\"", e, escape_string(manf))?;
            }
        }
        writeln!(f, "{}", self.append_rc_content)?;
        Ok(())
    }

    /// Set a path to an already existing resource file.
    ///
    /// We will neither modify this file nor parse its contents. This function
    /// simply replaces the internally generated resource file that is passed to
    /// the compiler. You can use this function to write a resource file yourself.
    pub fn set_resource_file(&mut self, path: &str) -> &mut Self {
        self.rc_file = Some(path.to_string());
        self
    }

    /// Append an additional snippet to the generated rc file.
    ///
    /// # Example
    ///
    /// Define a menu resource:
    ///
    /// ```rust
    /// # extern crate winres;
    /// # if cfg!(target_os = "windows") {
    ///     let mut res = winres::WindowsResource::new();
    ///     res.append_rc_content(r##"sample MENU
    /// {
    ///     MENUITEM "&Soup", 100
    ///     MENUITEM "S&alad", 101
    ///     POPUP "&Entree"
    ///     {
    ///          MENUITEM "&Fish", 200
    ///          MENUITEM "&Chicken", 201, CHECKED
    ///          POPUP "&Beef"
    ///          {
    ///               MENUITEM "&Steak", 301
    ///               MENUITEM "&Prime Rib", 302
    ///          }
    ///     }
    ///     MENUITEM "&Dessert", 103
    /// }"##);
    /// #    res.compile()?;
    /// # }
    /// # Ok::<_, std::io::Error>(())
    /// ```
    pub fn append_rc_content(&mut self, content: &str) -> &mut Self {
        if !(self.append_rc_content.ends_with('\n') || self.append_rc_content.is_empty()) {
            self.append_rc_content.push('\n');
        }
        self.append_rc_content.push_str(content);
        self
    }

    /// Override the output directory.
    ///
    /// As a default, we use `%OUT_DIR%` set by cargo, but it may be necessary to override the
    /// the setting.
    pub fn set_output_directory(&mut self, path: &str) -> &mut Self {
        self.output_directory = path.to_string();
        self
    }

    /// Print the links or not
    pub fn set_link(&mut self, value: bool) -> &mut Self {
        self.link = value;
        self
    }

    /// Run the resource compiler
    ///
    /// This function generates a resource file from the settings or
    /// uses an existing resource file and passes it to the resource compiler
    /// of your toolkit.
    ///
    /// Further more we will print the correct statements for
    /// `cargo:rustc-link-lib=` and `cargo:rustc-link-search` on the console,
    /// so that the cargo build script can link the compiled resource file.
    pub fn compile(&mut self) -> io::Result<()> {
        let output = PathBuf::from(&self.output_directory);
        let rc = output.join("resource.rc");
        if self.rc_file.is_none() {
            self.write_resource_file(&rc)?;
        }
        let rc = if let Some(s) = self.rc_file.as_ref() {
            s.clone()
        } else {
            rc.to_str().unwrap().to_string()
        };

        if cfg!(target_env = "msvc") {
            self.compile_with_toolkit_msvc(rc.as_str())
        } else if cfg!(target_env = "gnu") {
            self.compile_with_toolkit_gnu(rc.as_str())
        } else {
            Err(io::Error::new(
                io::ErrorKind::Other,
                "Can only compile resource file when target_env is \"gnu\" or \"msvc\"",
            ))
        }
    }

    fn compile_with_toolkit_gnu(&mut self, input: &str) -> io::Result<()> {
        let output = PathBuf::from(&self.output_directory).join("resource.o");
        let input = PathBuf::from(input);
        let manifest = match &self.assets_path {
            Some(val) => val,
            None => &std::env::var("CARGO_MANIFEST_DIR").unwrap(),
        };

        let status = process::Command::new(&self.windres_path)
            .current_dir(&self.toolkit_path)
            .arg(format!("-I{}", manifest))
            .arg(format!("{}", input.display()))
            .arg(format!("{}", output.display()))
            .status()?;
        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Could not compile resource file",
            ));
        }

        let libname = PathBuf::from(&self.output_directory).join("libresource.a");
        let status = process::Command::new(&self.ar_path)
            .current_dir(&self.toolkit_path)
            .arg("rsc")
            .arg(format!("{}", libname.display()))
            .arg(format!("{}", output.display()))
            .status()?;
        if !status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Could not create static library for resource file",
            ));
        }

        self.linker.lib = "static=resource".to_string();
        self.linker.path = self.output_directory.clone();
        self.linker.files = vec![
            output.to_string_lossy().to_string(),
            libname.to_string_lossy().to_string(),
        ];

        if self.link {
            println!("cargo:rustc-link-search=native={}", self.output_directory);
            println!("cargo:rustc-link-lib=static=resource");
        }

        Ok(())
    }

    fn compile_with_toolkit_msvc(&mut self, input: &str) -> io::Result<()> {
        let rc_exe = PathBuf::from(&self.toolkit_path).join("rc.exe");
        let rc_exe = if !rc_exe.exists() {
            if cfg!(target_arch = "x86_64") {
                PathBuf::from(&self.toolkit_path).join(r"bin\x64\rc.exe")
            } else {
                PathBuf::from(&self.toolkit_path).join(r"bin\x86\rc.exe")
            }
        } else {
            rc_exe
        };
        let manifest = match &self.assets_path {
            Some(val) => val,
            None => &std::env::var("CARGO_MANIFEST_DIR").unwrap(),
        };

        println!("Selected RC path: '{}'", rc_exe.display());
        let output = PathBuf::from(&self.output_directory).join("resource.lib");
        let input = PathBuf::from(input);
        let mut command = process::Command::new(&rc_exe);
        let command = command.arg(format!("/I{}", manifest));

        if self.add_toolkit_include {
            let root = win_sdk_include_root(&rc_exe);
            println!("Adding toolkit include: {}", root.display());
            command.arg(format!("/I{}", root.join("um").display()));
            command.arg(format!("/I{}", root.join("shared").display()));
        }

        let status = command
            .arg(format!("/fo{}", output.display()))
            .arg(format!("{}", input.display()))
            .output()?;

        println!(
            "RC Output:\n{}\n------",
            String::from_utf8_lossy(&status.stdout)
        );
        println!(
            "RC Error:\n{}\n------",
            String::from_utf8_lossy(&status.stderr)
        );
        if !status.status.success() {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Could not compile resource file",
            ));
        }

        self.linker.lib = "dylib=resource".to_string();
        self.linker.path = self.output_directory.clone();
        self.linker.files = vec![output.to_string_lossy().to_string()];

        if self.link {
            println!("cargo:rustc-link-search=native={}", self.output_directory);
            println!("cargo:rustc-link-lib=dylib=resource");
        }

        Ok(())
    }
}

/// Find a Windows SDK
fn get_sdk() -> io::Result<Vec<PathBuf>> {
    // use the reg command, so we don't need a winapi dependency
    let output = process::Command::new("reg")
        .arg("query")
        .arg(r"HKLM\SOFTWARE\Microsoft\Windows Kits\Installed Roots")
        .arg("/reg:32")
        .output()?;

    if !output.status.success() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            format!(
                "Querying the registry failed with error message:\n{}",
                String::from_utf8(output.stderr)
                    .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?
            ),
        ));
    }

    let lines = String::from_utf8(output.stdout)
        .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))?;
    let mut kits: Vec<PathBuf> = Vec::new();
    let mut lines: Vec<&str> = lines.lines().collect();
    lines.reverse();
    for line in lines {
        if line.trim().starts_with("KitsRoot") {
            let kit: String = line
                .chars()
                .skip(line.find("REG_SZ").unwrap() + 6)
                .skip_while(|c| c.is_whitespace())
                .collect();

            let p = PathBuf::from(&kit);
            let rc = if cfg!(target_arch = "x86_64") {
                p.join(r"bin\x64\rc.exe")
            } else {
                p.join(r"bin\x86\rc.exe")
            };

            if rc.exists() {
                println!("{:?}", rc);
                kits.push(rc.parent().unwrap().to_owned());
            }

            if let Ok(bin) = p.join("bin").read_dir() {
                for e in bin.filter_map(|e| e.ok()) {
                    let p = if cfg!(target_arch = "x86_64") {
                        e.path().join(r"x64\rc.exe")
                    } else {
                        e.path().join(r"x86\rc.exe")
                    };
                    if p.exists() {
                        println!("{:?}", p);
                        kits.push(p.parent().unwrap().to_owned());
                    }
                }
            }
        }
    }
    if kits.is_empty() {
        return Err(io::Error::new(
            io::ErrorKind::Other,
            "Can not find Windows SDK",
        ));
    }

    Ok(kits)
}

pub(crate) fn escape_string(string: &str) -> String {
    let mut escaped = String::new();
    for chr in string.chars() {
        // In quoted RC strings, double-quotes are escaped by using two
        // consecutive double-quotes.  Other characters are escaped in the
        // usual C way using backslashes.
        match chr {
            '"' => escaped.push_str("\"\""),
            '\'' => escaped.push_str("\\'"),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\t' => escaped.push_str("\\t"),
            '\r' => escaped.push_str("\\r"),
            _ => escaped.push(chr),
        };
    }
    escaped
}

fn win_sdk_include_root(path: &Path) -> PathBuf {
    let mut tools_path = PathBuf::new();
    let mut iter = path.iter();
    while let Some(p) = iter.next() {
        if p == "bin" {
            let version = iter.next().unwrap();
            tools_path.push("Include");
            if version.to_string_lossy().starts_with("10.") {
                tools_path.push(version);
            }
            break;
        } else {
            tools_path.push(p);
        }
    }

    tools_path
}
