use super::*;
use object::{Architecture as ObjectArchitecture, BinaryFormat};
use tempfile::tempdir;

fn native_object(symbols: &[(&str, u64)], format: BinaryFormat) -> Vec<u8> {
    let mut obj =
        object::write::Object::new(format, ObjectArchitecture::X86_64, Endianness::Little);
    let text = obj.section_id(StandardSection::Text);
    obj.append_section_data(text, &[0xc3; 32], 16);
    for &(name, address) in symbols {
        obj.add_symbol(Symbol {
            name: name.as_bytes().to_vec(),
            value: address,
            size: 1,
            kind: SymbolKind::Text,
            scope: SymbolScope::Linkage,
            weak: false,
            section: SymbolSection::Section(text),
            flags: SymbolFlags::None,
        });
    }
    obj.write().unwrap()
}

#[test]
fn native_jump_table_preserves_merged_aliases() {
    let dir = tempdir().unwrap();
    for (format, target) in [
        (BinaryFormat::MachO, "x86_64-apple-darwin"),
        (BinaryFormat::Elf, "x86_64-unknown-linux-gnu"),
    ] {
        let triple = target.parse().unwrap();
        let base = dir.path().join("base");
        let patch = dir.path().join("patch");
        std::fs::write(
            &base,
            native_object(&[("main", 0), ("foo", 8), ("bar", 16)], format),
        )
        .unwrap();
        std::fs::write(
            &patch,
            native_object(&[("main", 0), ("foo", 8), ("bar", 8)], format),
        )
        .unwrap();
        let cache = HotpatchModuleCache::new(&base, &triple).unwrap();
        let table = create_native_jump_table(&patch, &triple, &cache).unwrap();
        assert_eq!(table.map.get(&8), table.map.get(&16));
        assert!(table.map.contains_key(&16));
    }
}

#[test]
fn native_jump_table_rejects_diverging_aliases() {
    let dir = tempdir().unwrap();
    let triple = "x86_64-unknown-linux-gnu".parse().unwrap();
    let base = dir.path().join("base");
    let patch = dir.path().join("patch");
    std::fs::write(
        &base,
        native_object(&[("main", 0), ("foo", 8), ("bar", 8)], BinaryFormat::Elf),
    )
    .unwrap();
    std::fs::write(
        &patch,
        native_object(&[("main", 0), ("foo", 8), ("bar", 16)], BinaryFormat::Elf),
    )
    .unwrap();
    let cache = HotpatchModuleCache::new(&base, &triple).unwrap();
    assert!(create_native_jump_table(&patch, &triple, &cache).is_err());
}

#[test]
fn duplicate_native_symbol_names_are_rejected() {
    let dir = tempdir().unwrap();
    let triple = "x86_64-unknown-linux-gnu".parse().unwrap();
    let base = dir.path().join("base");
    let patch = dir.path().join("patch");
    let duplicate = native_object(
        &[("main", 0), ("local", 8), ("local", 16)],
        BinaryFormat::Elf,
    );
    let unambiguous = native_object(&[("main", 0), ("local", 8)], BinaryFormat::Elf);
    std::fs::write(&base, &duplicate).unwrap();
    std::fs::write(&patch, &unambiguous).unwrap();
    let cache = HotpatchModuleCache::new(&base, &triple).unwrap();
    assert!(cache.ambiguous_symbols.contains("local"));
    assert!(create_native_jump_table(&patch, &triple, &cache).is_err());
    std::fs::write(&base, &unambiguous).unwrap();
    std::fs::write(&patch, &duplicate).unwrap();
    let cache = HotpatchModuleCache::new(&base, &triple).unwrap();
    assert!(create_native_jump_table(&patch, &triple, &cache).is_err());
}

#[test]
fn elf_tls_preserves_section_alignment() {
    let dir = tempdir().unwrap();
    let triple = "x86_64-unknown-linux-gnu".parse().unwrap();
    let mut base = object::write::Object::new(
        BinaryFormat::Elf,
        ObjectArchitecture::X86_64,
        Endianness::Little,
    );
    let text = base.section_id(StandardSection::Text);
    let tls = base.section_id(StandardSection::Tls);
    for (name, section, data, alignment, kind) in [
        ("main", text, vec![0xc3], 1, SymbolKind::Text),
        ("aligned_tls", tls, vec![1; 64], 64, SymbolKind::Tls),
    ] {
        let symbol = base.add_symbol(Symbol {
            name: name.as_bytes().to_vec(),
            value: 0,
            size: 0,
            kind,
            scope: SymbolScope::Linkage,
            weak: false,
            section: SymbolSection::Undefined,
            flags: SymbolFlags::None,
        });
        base.add_symbol_data(symbol, section, &data, alignment);
    }
    let base_path = dir.path().join("base");
    std::fs::write(&base_path, base.write().unwrap()).unwrap();
    let mut consumer = object::write::Object::new(
        BinaryFormat::Elf,
        ObjectArchitecture::X86_64,
        Endianness::Little,
    );
    consumer.add_symbol(Symbol {
        name: b"aligned_tls".to_vec(),
        value: 0,
        size: 0,
        kind: SymbolKind::Tls,
        scope: SymbolScope::Unknown,
        weak: false,
        section: SymbolSection::Undefined,
        flags: SymbolFlags::None,
    });
    let consumer_path = dir.path().join("consumer.o");
    std::fs::write(&consumer_path, consumer.write().unwrap()).unwrap();
    let cache = HotpatchModuleCache::new(&base_path, &triple).unwrap();
    let bytes = create_undefined_symbol_stub(&cache, &[consumer_path], &triple, 0x10000).unwrap();
    let stub = File::parse(bytes.as_slice()).unwrap();
    assert_eq!(stub.section_by_name(".tdata").unwrap().align(), 64);
}

#[cfg(target_os = "macos")]
#[test]
fn macho_tls_uses_original_storage_and_relocated_initializers() {
    use std::ffi::{CString, c_char, c_void};
    use std::process::Command;

    unsafe extern "C" {
        fn dlopen(path: *const c_char, flags: i32) -> *mut c_void;
        fn dlsym(handle: *mut c_void, name: *const c_char) -> *mut c_void;
    }

    fn compile(args: &[&str], dir: &Path) {
        let output = Command::new("clang")
            .current_dir(dir)
            .args(args)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    fn load(path: &Path) -> *mut c_void {
        let path = CString::new(path.to_str().unwrap()).unwrap();
        let handle = unsafe { dlopen(path.as_ptr(), 2) };
        assert!(!handle.is_null());
        handle
    }

    fn symbol(handle: *mut c_void, name: &str) -> usize {
        let name = CString::new(name).unwrap();
        let address = unsafe { dlsym(handle, name.as_ptr()) };
        assert!(!address.is_null());
        address as usize
    }

    let dir = tempdir().unwrap();
    std::fs::write(
        dir.path().join("base.c"),
        r#"
long global_value = 7;
__thread long zero_tls[32];
__thread long *pointer_tls = &global_value;
__thread unsigned char aligned_tls[64] __attribute__((aligned(64))) = {1};
long *zero_address(void) { return zero_tls; }
long *pointer_value(void) { return pointer_tls; }
void *aligned_address(void) { return aligned_tls; }
int main(void) { return 0; }
"#,
    )
    .unwrap();
    std::fs::write(
        dir.path().join("consumer.c"),
        r#"
extern __thread long zero_tls[32];
extern __thread long *pointer_tls;
extern __thread unsigned char aligned_tls[64];
long *zero_address(void) { return zero_tls; }
long *pointer_value(void) { return pointer_tls; }
void *aligned_address(void) { return aligned_tls; }
"#,
    )
    .unwrap();
    compile(&["-dynamiclib", "base.c", "-o", "base.dylib"], dir.path());
    compile(&["-O2", "-c", "consumer.c", "-o", "consumer.o"], dir.path());
    let base_path = dir.path().join("base.dylib");
    let base = load(&base_path);
    let triple = Triple::host();
    let cache = HotpatchModuleCache::new(&base_path, &triple).unwrap();
    let stub = create_undefined_symbol_stub(
        &cache,
        &[dir.path().join("consumer.o")],
        &triple,
        symbol(base, "main") as u64,
    )
    .unwrap();
    std::fs::write(dir.path().join("stub.o"), stub).unwrap();
    compile(
        &["-dynamiclib", "consumer.o", "stub.o", "-o", "patch.dylib"],
        dir.path(),
    );
    let patch = load(&dir.path().join("patch.dylib"));
    let functions: Vec<_> = ["zero_address", "pointer_value", "aligned_address"]
        .into_iter()
        .map(|name| (symbol(base, name), symbol(patch, name)))
        .collect();
    let check = move || {
        for &(old, new) in &functions {
            let old: unsafe extern "C" fn() -> usize = unsafe { std::mem::transmute(old) };
            let new: unsafe extern "C" fn() -> usize = unsafe { std::mem::transmute(new) };
            assert_eq!(unsafe { old() }, unsafe { new() });
        }
    };
    check();
    std::thread::spawn(check).join().unwrap();
}

#[cfg(target_os = "macos")]
#[test]
fn elf_tls_initializers_apply_dynamic_relocations() {
    use std::process::Command;
    let dir = tempdir().unwrap();
    let libdir = Command::new("rustc")
        .args(["--print", "target-libdir"])
        .output()
        .unwrap();
    let libdir = PathBuf::from(String::from_utf8(libdir.stdout).unwrap().trim());
    let linker = libdir.parent().unwrap().join("bin/rust-lld");
    std::fs::write(dir.path().join("base.c"), "long global_value = 7; __thread long *pointer_tls[3] = { &global_value, &global_value, &global_value }; int main(void) { return 0; }").unwrap();
    std::fs::write(
        dir.path().join("consumer.c"),
        "extern __thread long *pointer_tls[3]; long *get_pointer(void) { return pointer_tls[2]; }",
    )
    .unwrap();
    for target in ["x86_64-unknown-linux-gnu", "aarch64-unknown-linux-gnu"] {
        for source in ["base", "consumer"] {
            let output = Command::new("clang")
                .current_dir(dir.path())
                .args([
                    "-target",
                    target,
                    "-fPIC",
                    "-c",
                    &format!("{source}.c"),
                    "-o",
                    &format!("{source}.o"),
                ])
                .output()
                .unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
        }
        for (symbolic, packed) in [(false, false), (true, false), (true, true)] {
            let mut command = Command::new(&linker);
            command
                .current_dir(dir.path())
                .args(["-flavor", "gnu", "-shared", "base.o", "-o", "base.so"]);
            if symbolic {
                command.arg("-Bsymbolic");
            }
            if packed {
                command.arg("--pack-dyn-relocs=relr");
            }
            let output = command.output().unwrap();
            assert!(
                output.status.success(),
                "{}",
                String::from_utf8_lossy(&output.stderr)
            );
            let triple = target.parse().unwrap();
            let cache = HotpatchModuleCache::new(&dir.path().join("base.so"), &triple).unwrap();
            let slide = 0x200000;
            let bytes = create_undefined_symbol_stub(
                &cache,
                &[dir.path().join("consumer.o")],
                &triple,
                cache.symbol_table["main"].address + slide,
            )
            .unwrap();
            let stub = File::parse(bytes.as_slice()).unwrap();
            let data = stub.section_by_name(".tdata").unwrap().data().unwrap();
            assert_eq!(data.len(), 24);
            for pointer in data.chunks_exact(8) {
                assert_eq!(
                    u64::from_le_bytes(pointer.try_into().unwrap()),
                    cache.symbol_table["global_value"].address + slide
                );
            }
        }
    }
}

#[test]
fn malformed_wasm_metadata_is_not_silently_ignored() {
    assert!(parse_bytes_to_data_segment(b"\0asm\x01\0\0\0\x0b\x80").is_err());
}

#[test]
fn wasm_layout_preserves_bss_and_alignment() {
    let mut module = Module::default();
    module.customs.add(walrus::RawCustomSection {
        name: "dylink.0".into(),
        data: vec![1, 6, 0x84, 0x80, 0x40, 20, 7, 2],
    });
    let bytes = module.emit_wasm();
    let layout = wasm_patch_layout(&module, &bytes).unwrap();
    assert_eq!(layout.memory_size, 1048580);
    assert_eq!(layout.memory_alignment, 20);
    assert_eq!(layout.table_size, 7);
    assert_eq!(layout.table_alignment, 2);
    let (pages, _) = layout.reservation().unwrap();
    assert!(pages >= 17);
}

#[test]
fn wasm_memory_requires_linker_layout() {
    let mut module = Module::default();
    module.add_import_memory("env", "memory", false, false, 0, None, None);
    let bytes = module.emit_wasm();
    assert!(wasm_patch_layout(&module, &bytes).is_err());
}

#[test]
fn wasm_patch_without_elements_is_rewritable() {
    let dir = tempdir().unwrap();
    let mut module = Module::default();
    let mut builder = FunctionBuilder::new(&mut module.types, &[], &[]);
    builder.name("__wasm_apply_global_relocs".into());
    let function = builder.finish(vec![], &mut module.funcs);
    module.exports.add("__wasm_apply_global_relocs", function);
    let path = dir.path().join("patch.wasm");
    std::fs::write(&path, module.emit_wasm()).unwrap();
    let mut old = Module::default();
    let cache = HotpatchModuleCache {
        old_bytes: old.emit_wasm(),
        old_wasm: old,
        ..Default::default()
    };
    for _ in 0..2 {
        let table = create_wasm_jump_table(&path, &cache).unwrap();
        assert_eq!(table.ifunc_count, 0);
        let bytes = std::fs::read(&path).unwrap();
        wasmparser::Validator::new().validate_all(&bytes).unwrap();
    }
}

#[test]
fn native_stubs_preserve_absolute_symbols_and_unix_import_prefixes() {
    let dir = tempdir().unwrap();
    let triple = "x86_64-unknown-linux-gnu".parse().unwrap();
    let base = dir.path().join("base");
    std::fs::write(
        &base,
        native_object(&[("main", 0), ("__imp_static", 8)], BinaryFormat::Elf),
    )
    .unwrap();
    let mut cache = HotpatchModuleCache::new(&base, &triple).unwrap();
    let data = cache.symbol_table.get_mut("__imp_static").unwrap();
    data.kind = SymbolKind::Data;
    data.is_absolute = true;
    data.flags = SymbolFlags::None;
    let mut consumer = object::write::Object::new(
        BinaryFormat::Elf,
        ObjectArchitecture::X86_64,
        Endianness::Little,
    );
    consumer.add_symbol(Symbol {
        name: b"__imp_static".to_vec(),
        value: 0,
        size: 0,
        kind: SymbolKind::Data,
        scope: SymbolScope::Unknown,
        weak: false,
        section: SymbolSection::Undefined,
        flags: SymbolFlags::None,
    });
    let path = dir.path().join("consumer.o");
    std::fs::write(&path, consumer.write().unwrap()).unwrap();
    let bytes = create_undefined_symbol_stub(&cache, std::slice::from_ref(&path), &triple, 0x10000)
        .unwrap();
    let stub = File::parse(bytes.as_slice()).unwrap();
    assert_eq!(stub.symbol_by_name("__imp_static").unwrap().address(), 8);
    let data = cache.symbol_table.get_mut("__imp_static").unwrap();
    data.kind = SymbolKind::Text;
    data.flags = SymbolFlags::Elf {
        st_info: object::elf::STT_GNU_IFUNC,
        st_other: 0,
    };
    assert!(create_undefined_symbol_stub(&cache, &[path], &triple, 0x10000).is_err());
}

#[test]
fn wasm_table_size_counts_duplicate_and_unnamed_slots() {
    let dir = tempdir().unwrap();
    let mut module = Module::default();
    let (table, _) = module.add_import_table(
        "env",
        "__indirect_function_table",
        false,
        0,
        None,
        walrus::RefType::Funcref,
    );
    let (base, _) =
        module.add_import_global("env", "__table_base", walrus::ValType::I32, false, false);
    let mut builder = FunctionBuilder::new(&mut module.types, &[], &[]);
    builder.name("duplicate".to_string());
    let function = builder.finish(vec![], &mut module.funcs);
    let unnamed =
        FunctionBuilder::new(&mut module.types, &[], &[]).finish(vec![], &mut module.funcs);
    module.elements.add(
        ElementKind::Active {
            table,
            offset: ConstExpr::Global(base),
        },
        ElementItems::Functions(vec![function, function, unnamed]),
    );
    let path = dir.path().join("patch.wasm");
    std::fs::write(&path, module.emit_wasm()).unwrap();
    let mut old = Module::default();
    let cache = HotpatchModuleCache {
        old_bytes: old.emit_wasm(),
        old_wasm: old,
        ..Default::default()
    };
    let patch = create_wasm_jump_table(&path, &cache).unwrap();
    assert_eq!(patch.ifunc_count, 3);
}
