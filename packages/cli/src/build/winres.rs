#![allow(unused)]
//! Modified version of <https://github.com/BenjaminRi/winresource> or <https://github.com/tauri-apps/winres>
//!
//! Rust Windows resource helper
//!
//! This crate implements a simple generator for Windows resource (.rc) files
//! for use with either Microsoft `rc.exe` resource compiler or with GNU `windres.exe`
//!
//! Maybe it could be replaced by <https://crates.io/crates/windows-resource> in the future
//! if it's going to be used to compile resources as the name suggests instead of external tools

use dioxus_html::tr;
use krates::semver::Version;

use anyhow::{anyhow, Context, Result};
use std::collections::HashMap;
use std::env;
use std::fs::File;
use std::io;
use std::io::prelude::*;
use std::path::{Path, PathBuf};
use std::process;
use target_lexicon::{Architecture, Environment, OperatingSystem, Triple, X86_32Architecture};

const DEFAULT_ICON: &[u8] = include_bytes!("../../assets/icon.ico");

pub(crate) fn write_default_icon(output_dir: &Path) -> Result<PathBuf> {
    let icon = output_dir.join("icon.ico");
    let mut file = File::create(&icon)?;
    file.write_all(DEFAULT_ICON)?;
    Ok(icon)
}

/// Values based on <https://learn.microsoft.com/en-us/windows/win32/menurc/about-icons>
/// use to_str()
#[allow(clippy::upper_case_acronyms)]
#[derive(PartialEq, Eq, Hash, Debug, Clone, Copy)]
pub enum IDI {
    APPLICATION = 32512,
    ERROR = 32513,
    QUESTION = 32514,
    WARNING = 32515,
    INFORMATION = 32516,
    WINLOGO = 32517,
    SHIELD = 32518,
}

impl std::fmt::Display for IDI {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", *self as u32)
    }
}

impl From<IDI> for String {
    fn from(e: IDI) -> Self {
        e.to_string()
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

/// Common properties fields
/// <https://learn.microsoft.com/en-us/windows/win32/menurc/versioninfo-resource#string-name>
#[derive(PartialEq, Eq, Hash, Debug)]
pub enum Properties {
    FileVersion,
    FileDescription,
    ProductVersion,
    ProductName,
    OriginalFilename,
    Copyright,
    Trademark,
    CompanyName,
    Comments,
    InternalName,
    /// Additionally there exists
    /// `"PrivateBuild"`, `"SpecialBuild"`
    /// which should only be set, when the `FILEFLAGS` property is set to
    /// `VS_FF_PRIVATEBUILD(0x08)` or `VS_FF_SPECIALBUILD(0x20)`
    ///
    /// It is possible to use arbitrary field names but Windows Explorer and other
    /// tools might not show them.
    Other(&'static str),
}

impl std::fmt::Display for Properties {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Properties::FileVersion => write!(f, "FileVersion"),
            Properties::FileDescription => write!(f, "FileDescription"),
            Properties::ProductVersion => write!(f, "ProductVersion"),
            Properties::ProductName => write!(f, "ProductName"),
            Properties::OriginalFilename => write!(f, "OriginalFilename"),
            Properties::Copyright => write!(f, "LegalCopyright"),
            Properties::Trademark => write!(f, "LegalTrademark"),
            Properties::CompanyName => write!(f, "CompanyName"),
            Properties::Comments => write!(f, "Comments"),
            Properties::InternalName => write!(f, "InternalName"),
            Properties::Other(other) => write!(f, "{}", other),
        }
    }
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
}

#[derive(Debug)]
pub struct WindowsResource {
    properties: HashMap<String, String>,
    version_info: HashMap<VersionInfo, u64>,
    icons: Vec<Icon>,
    language: u16,
    add_toolkit_include: bool,
    append_rc_content: String,
}

impl WindowsResource {
    /// Create a new resource with version info struct
    ///
    /// We initialize the resource file with values provided in Dioxus.toml
    ///
    pub fn new(version: u64, file_version: u64) -> Self {
        let props: HashMap<String, String> = HashMap::new();
        let mut ver: HashMap<VersionInfo, u64> = HashMap::new();

        ver.insert(VersionInfo::FILEVERSION, file_version);
        ver.insert(VersionInfo::PRODUCTVERSION, version);
        ver.insert(VersionInfo::FILEOS, 0x00040004); // VOS_NT_WINDOWS32 (0x40004)
        ver.insert(VersionInfo::FILETYPE, 1); // VFT_APP (0x1)
        ver.insert(VersionInfo::FILESUBTYPE, 0); // VFT2_UNKNOWN (0x0)
        ver.insert(VersionInfo::FILEFLAGSMASK, 0x3F); // VS_FFI_FILEFLAGSMASK (0x3F)
        ver.insert(VersionInfo::FILEFLAGS, 0); // 0x0

        WindowsResource {
            properties: props,
            version_info: ver,
            icons: Vec::new(),
            language: 0,
            add_toolkit_include: false,
            append_rc_content: String::new(),
        }
    }

    /// Set string properties of the version info struct.
    ///
    /// See [`Properties`] for valid values
    pub fn set(&mut self, name: Properties, value: &str) -> &mut Self {
        self.properties.insert(name.to_string(), value.to_string());
        self
    }

    /// Set the user interface language of the file
    ///
    /// For possible values look at the `winapi::um::winnt` constants, specifically those
    /// starting with `LANG_` and `SUBLANG_` or at <https://learn.microsoft.com/en-us/windows/win32/menurc/versioninfo-resource#langID>
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
    /// Equivalent ```to set_icon_with_id(path, IDI::APPLICATION)```.
    pub fn set_icon(&mut self, path: PathBuf) -> &mut Self {
        self.set_icon_with_id(path, IDI::APPLICATION)
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
    /// ```
    /// see [`IDI`] for special icons ids
    pub fn set_icon_with_id(&mut self, path: PathBuf, name_id: impl Into<String>) -> &mut Self {
        self.icons.push(Icon {
            path: path.to_string_lossy().to_string(),
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

    /// Set the path to the ar executable.
    pub fn add_toolkit_include(&mut self, add: bool) -> &mut Self {
        self.add_toolkit_include = add;
        self
    }

    /// Write a resource file with the set values
    fn write_resource_file(&self, path: &Path) -> Result<PathBuf> {
        let path = path.join("resource.rc");
        let mut f = File::create(&path)?;

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

        writeln!(f, "{}", self.append_rc_content)?;
        Ok(path)
    }

    /// Append an additional snippet to the generated rc file.
    pub fn append_rc_content(&mut self, content: &str) -> &mut Self {
        if !(self.append_rc_content.ends_with('\n') || self.append_rc_content.is_empty()) {
            self.append_rc_content.push('\n');
        }
        self.append_rc_content.push_str(content);
        self
    }

    /// Run the resource compiler
    ///
    /// This function generates a resource file from the settings or
    /// uses an existing resource file and passes it to the resource compiler
    /// of your toolkit.
    pub fn compile(&mut self, target: &Triple, output_dir: &Path) -> Result<WindowsResourceLinker> {
        if matches!(target.environment, Environment::Msvc) {
            tracing::debug!("Compiling Windows resource file with msvc toolkit");
            self.compile_with_toolkit_msvc(target, output_dir)
        } else if target.environment.to_string().contains("gnu") {
            tracing::debug!("Compiling Windows resource file with gnu toolkit");
            self.compile_with_toolkit_gnu(target, output_dir)
        } else {
            Err(anyhow!(
                "Can only compile resource file when target_env is 'gnu' or 'msvc'",
            ))
        }
    }

    fn compile_with_toolkit_gnu(
        &mut self,
        target: &Triple,
        output_dir: &Path,
    ) -> Result<WindowsResourceLinker> {
        let toolkit_path =
            if env::var_os("HOST").is_some_and(|v| v.to_string_lossy().contains("windows")) {
                PathBuf::from("\\")
            } else {
                PathBuf::from("/")
            };

        let prefix = if env::var_os("HOST")
            .zip(env::var_os("TARGET"))
            .is_some_and(|(h, t)| h != t)
        {
            match (target.architecture, target.environment) {
                // Standard MinGW-w64 targets
                (Architecture::X86_64, Environment::Gnu) => "x86_64-w64-mingw32-",
                (Architecture::X86_32(X86_32Architecture::I686), Environment::Gnu) => {
                    "i686-w64-mingw32-"
                }
                (Architecture::X86_32(X86_32Architecture::I586), Environment::Gnu) => {
                    "i586-w64-mingw32-"
                }
                // MinGW supports ARM64 only with an LLVM-based toolchain
                // (x86 users might also be using LLVM, but we can't tell that from the Rust target...)
                (Architecture::Aarch64(_), Environment::Gnu) => "llvm-",

                // LLVM-based MinGW targets (gnullvm) always use llvm- prefix
                (_, Environment::GnuLlvm) => "llvm-",
                // fail safe
                _ => {
                    tracing::warn!("Unknown Windows target used for cross-compilation - invoking unprefixed windres");
                    ""
                }
            }
        } else {
            ""
        };

        // this could probably be done with variables in Dioxus.toml instead of env::var
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

        let rc_file = self.write_resource_file(output_dir)?;

        tracing::debug!("Input file: '{}'", rc_file.display());
        let output = output_dir.join("resource.o");
        tracing::debug!("Output object file: '{}'", output.display());

        tracing::debug!("Selected toolkit path: '{}'", &toolkit_path.display());
        tracing::debug!("Selected windres path: '{:?}'", &windres_path);
        tracing::debug!("Selected ar path: '{:?}'", &ar_path);

        let status = process::Command::new(&windres_path)
            .current_dir(&toolkit_path)
            .arg(format!("{}", rc_file.display()))
            .arg(format!("{}", output.display()))
            .output()?;

        if !status.status.success() {
            return Err(anyhow!("Compiling resource file {:?}", &status.stderr));
        }

        let libname = output_dir.join("libresource.a");
        tracing::debug!("Output lib file: '{}'", output.display());
        let status = process::Command::new(&ar_path)
            .current_dir(&toolkit_path)
            .arg("rsc")
            .arg(format!("{}", libname.display()))
            .arg(format!("{}", output.display()))
            .output()?;

        if !status.status.success() {
            return Err(anyhow!(
                "Creating static library for resource file {:?}",
                &status.stderr
            ));
        }

        Ok(WindowsResourceLinker {
            lib: "static=resource".to_string(),
            path: output_dir.to_string_lossy().to_string(),
            files: vec![
                output.to_string_lossy().to_string(),
                libname.to_string_lossy().to_string(),
            ],
        })
    }

    fn compile_with_toolkit_msvc(
        &mut self,
        target: &Triple,
        output_dir: &Path,
    ) -> Result<WindowsResourceLinker> {
        // The path to this could also be provided via Dioxus.toml if someone has the exe in other places
        let toolkit = get_sdk(matches!(target.architecture, Architecture::X86_64))?;

        let rc_file = self.write_resource_file(output_dir)?;

        tracing::debug!("Selected toolkit path: '{}'", toolkit.display());
        tracing::debug!("Input file: '{}'", rc_file.display());
        let output = output_dir.join("resource.lib");
        tracing::debug!("Output file: '{}'", output.display());
        let mut command = process::Command::new(&toolkit);

        if self.add_toolkit_include {
            let root = win_sdk_include_root(&toolkit);
            tracing::debug!("Adding toolkit include: {}", root.display());
            command.arg(format!("/I{}", root.join("um").display()));
            command.arg(format!("/I{}", root.join("shared").display()));
        }

        let status = command
            .arg(format!("/fo{}", output.display()))
            .arg(format!("{}", rc_file.display()))
            .output()?;

        if !status.status.success() {
            return Err(anyhow!("Compiling resource file {:?}", &status.stderr));
        }

        Ok(WindowsResourceLinker {
            lib: "dylib=resource".to_string(),
            path: output_dir.to_string_lossy().to_string(),
            files: vec![output.to_string_lossy().to_string()],
        })
    }
}

/// Find a Windows SDK
fn get_sdk(is_x64: bool) -> io::Result<PathBuf> {
    // use the reg command, so we don't need a winapi dependency
    let output = process::Command::new("reg")
        .arg("query")
        .arg(r"HKLM\SOFTWARE\Microsoft\Windows Kits\Installed Roots")
        .arg("/reg:32")
        .output()?;

    if !output.status.success() {
        return Err(io::Error::other(format!(
            "Querying the registry failed with error message:\n{}",
            String::from_utf8(output.stderr).map_err(|e| io::Error::other(e.to_string()))?
        )));
    }

    let lines = String::from_utf8(output.stdout).map_err(|e| io::Error::other(e.to_string()))?;
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
            let rc = if is_x64 {
                p.join(r"bin\x64\rc.exe")
            } else {
                p.join(r"bin\x86\rc.exe")
            };

            if rc.exists() {
                return Ok(rc);
            }

            if let Ok(bin) = p.join("bin").read_dir() {
                for e in bin.filter_map(|e| e.ok()) {
                    let p = if is_x64 {
                        e.path().join(r"x64\rc.exe")
                    } else {
                        e.path().join(r"x86\rc.exe")
                    };
                    if p.exists() {
                        return Ok(p);
                    }
                }
            }
        }
    }
    Err(io::Error::other("Can not find Windows SDK"))
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
