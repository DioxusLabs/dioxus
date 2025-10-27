//! The dioxus permission system.
//!
//! This module extracts permissions from compiled binaries and generates platform-specific
//! manifest files for platforms that require build-time permission declarations.
//!
//! Platforms requiring build-time manifests:
//! - Android: AndroidManifest.xml with <uses-permission> declarations
//! - iOS/macOS: Info.plist with usage description keys
//!
//! Other platforms (Linux, Web, Windows desktop) use runtime-only permissions
//! and do not require build-time manifest generation.

use std::{
    io::{Cursor, Read, Seek},
    path::Path,
};

use crate::Result;
use anyhow::Context;
use const_serialize::SerializeConst;
use object::{File, Object, ObjectSection, ObjectSymbol, ReadCache, ReadRef, Section, Symbol};
use permissions_core::{Permission, Platform};
use serde::Serialize;

const PERMISSION_SYMBOL_PREFIX: &str = "__PERMISSION__";

/// Android permission for Handlebars template
#[derive(Debug, Clone, Serialize)]
pub struct AndroidPermission {
    pub name: String,
    pub description: String,
}

/// iOS permission for Handlebars template
#[derive(Debug, Clone, Serialize)]
pub struct IosPermission {
    pub key: String,
    pub description: String,
}

/// macOS permission for Handlebars template
#[derive(Debug, Clone, Serialize)]
pub struct MacosPermission {
    pub key: String,
    pub description: String,
}

/// Extract permission symbols from the object file
fn permission_symbols<'a, 'b, R: ReadRef<'a>>(
    file: &'b File<'a, R>,
) -> impl Iterator<Item = (Symbol<'a, 'b, R>, Section<'a, 'b, R>)> + 'b {
    file.symbols()
        .filter(|symbol| {
            if let Ok(name) = symbol.name() {
                name.contains(PERMISSION_SYMBOL_PREFIX)
            } else {
                false
            }
        })
        .filter_map(move |symbol| {
            let section_index = symbol.section_index()?;
            let section = file.section_by_index(section_index).ok()?;
            Some((symbol, section))
        })
}

/// Find the offsets of any permission symbols in the given file.
/// 
/// Permissions are only extracted for Android/iOS/macOS builds which produce native binaries.
/// We only need to handle native object files (ELF/Mach-O).
fn find_symbol_offsets<'a, R: ReadRef<'a>>(
    _path: &Path,
    _file_contents: &[u8],
    file: &File<'a, R>,
) -> Result<Vec<u64>> {
    find_native_symbol_offsets(file)
}

/// Find the offsets of any permission symbols in a native object file.
fn find_native_symbol_offsets<'a, R: ReadRef<'a>>(file: &File<'a, R>) -> Result<Vec<u64>> {
    let mut offsets = Vec::new();
    for (symbol, section) in permission_symbols(file) {
        let virtual_address = symbol.address();

        let Some((section_range_start, _)) = section.file_range() else {
            tracing::error!(
                "Found __PERMISSION__ symbol {:?} in section {}, but the section has no file range",
                symbol.name(),
                section.index()
            );
            continue;
        };
        // Translate the section_relative_address to the file offset
        let section_relative_address: u64 = (virtual_address as i128 - section.address() as i128)
            .try_into()
            .expect("Virtual address should be greater than or equal to section address");
        let file_offset = section_range_start + section_relative_address;
        offsets.push(file_offset);
    }

    Ok(offsets)
}

/// Extract all permissions from the given file
pub(crate) fn extract_permissions_from_file(path: impl AsRef<Path>) -> Result<PermissionManifest> {
    let path = path.as_ref();
    let mut file = std::fs::File::open(path)?;

    let mut file_contents = Vec::new();
    file.read_to_end(&mut file_contents)?;
    let mut reader = Cursor::new(&file_contents);
    let read_cache = ReadCache::new(&mut reader);
    let object_file = object::File::parse(&read_cache)?;
    let offsets = find_symbol_offsets(path, &file_contents, &object_file)?;

    let mut permissions = Vec::new();

    for offset in offsets.iter().copied() {
        file.seek(std::io::SeekFrom::Start(offset))?;
        let mut data_in_range = vec![0; Permission::MEMORY_LAYOUT.size()];
        file.read_exact(&mut data_in_range)?;

        let buffer = const_serialize::ConstReadBuffer::new(&data_in_range);

        if let Some((_, permission)) = const_serialize::deserialize_const!(Permission, buffer) {
            tracing::debug!(
                "Found permission at offset {offset}: {:?} - {}",
                permission.kind(),
                permission.description()
            );
            permissions.push(permission);
        } else {
            tracing::warn!(
                "Found permission symbol at offset {offset} that could not be deserialized"
            );
        }
    }

    Ok(PermissionManifest::new(permissions))
}

/// A manifest of all permissions found in a binary
#[derive(Debug, Clone, Default)]
pub struct PermissionManifest {
    permissions: Vec<Permission>,
}

impl PermissionManifest {
    pub fn new(permissions: Vec<Permission>) -> Self {
        Self { permissions }
    }

    #[allow(dead_code)]
    pub fn permissions(&self) -> &[Permission] {
        &self.permissions
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.permissions.is_empty()
    }

    pub fn permissions_for_platform(&self, platform: Platform) -> Vec<&Permission> {
        self.permissions
            .iter()
            .filter(|p| p.supports_platform(platform))
            .collect()
    }
}

/// Get Android permissions for Handlebars template
pub(crate) fn get_android_permissions(manifest: &PermissionManifest) -> Vec<AndroidPermission> {
    manifest
        .permissions_for_platform(Platform::Android)
        .iter()
        .filter_map(|perm| {
            perm.android_permission()
                .map(|android_perm| AndroidPermission {
                    name: android_perm.to_string(),
                    description: perm.description().to_string(),
                })
        })
        .collect()
}

/// Get iOS permissions for Handlebars template
pub(crate) fn get_ios_permissions(manifest: &PermissionManifest) -> Vec<IosPermission> {
    manifest
        .permissions_for_platform(Platform::Ios)
        .iter()
        .filter_map(|perm| {
            perm.ios_key().map(|key| IosPermission {
                key: key.to_string(),
                description: perm.description().to_string(),
            })
        })
        .collect()
}

/// Get macOS permissions for Handlebars template
pub(crate) fn get_macos_permissions(manifest: &PermissionManifest) -> Vec<MacosPermission> {
    manifest
        .permissions_for_platform(Platform::Macos)
        .iter()
        .filter_map(|perm| {
            perm.macos_key().map(|key| MacosPermission {
                key: key.to_string(),
                description: perm.description().to_string(),
            })
        })
        .collect()
}

/// Check if permissions are needed for the platform
#[allow(dead_code)]
pub(crate) fn needs_permission_manifest(platform: Platform) -> bool {
    matches!(
        platform,
        Platform::Android | Platform::Ios | Platform::Macos
    )
}
