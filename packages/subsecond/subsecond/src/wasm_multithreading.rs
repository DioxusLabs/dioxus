#![cfg(feature = "experimental_wasm_multithreading_support")]
#![cfg(target_arch = "wasm32")]

use crate::PatchError::WasmRelated;
use crate::{commit_patch, PatchError};
use futures::lock::Mutex;
use js_sys::WebAssembly::{Memory, Module, Table};
use js_sys::{ArrayBuffer, Object, Promise, Reflect, Uint8Array, WebAssembly};
use leb128::read::Error;
use spin::MutexGuard;
use std::cell::Cell;
use std::collections::HashSet;
use std::io::Read;
use std::sync::atomic::{AtomicI32, AtomicUsize, Ordering};
use std::sync::{LazyLock, OnceLock};
use subsecond_types::JumpTable;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, Window, WorkerGlobalScope};
use crate::wasm_multithreading::CurrHotpatchingState::{Idle, MainThreadDynamicLinking, WebWorkersDynamicLinking};

static NEXT_THREAD_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq)]
struct MyThreadId(usize);

thread_local! {
    static IS_MAIN_THREAD: bool = web_sys::window().is_some();

    /// This thread id is for internally tracking what web worker haven't dynamically linked the patch
    static THREAD_ID: MyThreadId = MyThreadId(NEXT_THREAD_ID.fetch_add(1, Ordering::Relaxed));

    static CURR_WEB_WORKER_HOTPATCH_INITIALIZED: Cell<bool> = Cell::new(false);
}

struct HotpatchEntry {
    jump_table: JumpTable,
    table_base: u64,
    memory_base: u64,
}

enum CurrHotpatchingState {
    Idle,
    MainThreadDynamicLinking,
    WebWorkersDynamicLinking(WebWorkersDynamicLinkingState)
}

struct WebWorkersDynamicLinkingState {
    hotpatch_entry: HotpatchEntry,
    pending_thread_ids: HashSet<MyThreadId>,
}

struct GlobalHotpatchState {
    /// The hotpatches that have already been done.
    /// They will be re-dynamic-linked for each new web worker
    hotpatched: Vec<HotpatchEntry>,
    /// The hotpatch that's being done (waiting for web workers to dynamically link)
    curr_state: CurrHotpatchingState,
    /// When a new hotpatch comes before current hotpatch finishes, it's queued
    pending_hotpatches: Vec<JumpTable>,
    /// Collect all web worker thread ids
    worker_thread_ids: HashSet<MyThreadId>,
}

/// In Wasm the main thread cannot block, so use spinlock instead of mutex.
/// It will only lock briefly each time.
static GLOBAL_HOTPATCH_STATE: LazyLock<spin::Mutex<GlobalHotpatchState>> = LazyLock::new(|| {
    spin::Mutex::new(GlobalHotpatchState {
        hotpatched: Vec::new(),
        curr_state: Idle,
        pending_hotpatches: Vec::new(),
        worker_thread_ids: HashSet::new(),
    })
});

/// In WebAssembly multi-threading, applying patch cannot be done in one-shot function call.
/// Because currently the Wasm function table cannot be shared across threads.
/// Any dynamic linking requires each thread to cooperatively create new WebAssembly instance,
/// and apply changes to their own function table.
/// We must only change global jump table after all threads have dynamically linked the new code.
///
/// One-shot hotpatch in Wasm multithreading is possible after shared-everything-threads proposal,
/// which is still in early stage. https://github.com/WebAssembly/shared-everything-threads
pub async unsafe fn wasm_multithreaded_hotpatch_apply_begin(
    mut jump_table: JumpTable,
) -> Result<(), PatchError> {
    {
        let mut hotpatch_state = GLOBAL_HOTPATCH_STATE.lock();
        match hotpatch_state.curr_state {
            Idle => {
                hotpatch_state.curr_state = MainThreadDynamicLinking;
            }
            _ => {
                console::log_1(
                    &"Received new hotpatch when previous hotpatch hasn't finished. Queue it.".into(),
                );
                hotpatch_state.pending_hotpatches.push(jump_table);
                return Ok(());
            }
        }
    }

    let funcs: Table = wasm_bindgen::function_table().unchecked_into();
    let table_base = funcs.length();

    // the function addresses are relative. add them with table base to become absolute
    // in Wasm, function address means offset into function table
    for v in jump_table.map.values_mut() {
        *v += table_base as u64;
    }

    let module = load_wasm_module(&mut jump_table).await;

    let dylink_section_info = parse_dylink_section(&module).expect("Cannot parse dylink.0 section");

    console::log_1(
        &format!(
            "Patch binary data size {}",
            dylink_section_info.mem_info.memory_size
        )
        .into(),
    );

    const PAGE_SIZE: u32 = 64 * 1024;
    let page_count = dylink_section_info.mem_info.memory_size.div_ceil(PAGE_SIZE);
    let memory_base = (page_count + 1) * PAGE_SIZE;

    let memory: Memory = wasm_bindgen::memory().unchecked_into();
    memory.grow(page_count);

    let entry = HotpatchEntry {
        jump_table,
        table_base: table_base as u64,
        memory_base: memory_base as u64,
    };

    entry.internal_per_thread_dynamic_link(&module).await;

    {
        let mut hotpatch_state = GLOBAL_HOTPATCH_STATE.lock();

        assert!(matches!(hotpatch_state.curr_state, MainThreadDynamicLinking), "curr_state is not MainThreadDynamicLinking");

        hotpatch_state.curr_state = WebWorkersDynamicLinking(WebWorkersDynamicLinkingState {
            hotpatch_entry: entry,
            pending_thread_ids: hotpatch_state.worker_thread_ids.clone(),
        });

        notify_web_workers_to_dynamic_link();
    }

    Ok(())
}

fn notify_web_workers_to_dynamic_link() {
    todo!()
}

impl HotpatchEntry {
    unsafe fn apply_change_to_jump_table(&self) {
        unsafe { commit_patch(self.jump_table.clone()) };
    }

    async unsafe fn internal_per_thread_dynamic_link(&self, wasm_module: &Module) {
        let funcs: Table = wasm_bindgen::function_table().into();
        let exports: Object = wasm_bindgen::exports().into();

        let old_table_size = funcs.length();
        assert_eq!(
            old_table_size as u64, self.table_base,
            "The current threads' table size doesn't correspond to table_base. \
            Maybe due to \
            1. some race condition related to spawning new web worker during hotpatch\
            2. unexpectedly doing multiple hotpatches concurrently\
            3. new web worker doesn't do dynamic linking to previous patches correctly\
            4. other possible errors"
        );

        // We grow the ifunc table to accommodate the new functions
        // In theory we could just put all the ifuncs in the jump map and use that for our count,
        // but there's no guarantee from the jump table that it references "itself"
        // We might need a sentinel value for each ifunc in the jump map to indicate that it is
        funcs
            .grow(self.jump_table.ifunc_count as u32)
            .expect("growing table");

        // Build up the import object. We copy everything over from the current exports, but then
        // need to add in the memory and table base offsets for the relocations to work.
        //
        // let imports = {
        //     env: {
        //         memory: base.memory,
        //         __tls_base: base.__tls_base,
        //         __stack_pointer: base.__stack_pointer,
        //         __indirect_function_table: base.__indirect_function_table,
        //         __memory_base: memory_base,
        //         __table_base: table_base,
        //        ..base_exports
        //     },
        // };
        let env = Object::new();

        // Move memory, __tls_base, __stack_pointer, __indirect_function_table, and all exports over
        for key in Object::keys(&exports) {
            Reflect::set(
                &env,
                &key,
                &Reflect::get(&exports, &key).expect("getting field from exports"),
            )
            .expect("setting env");
        }

        // Set the memory and table in the imports
        // Following this pattern: Global.new({ value: "i32", mutable: false }, value)
        for (name, value) in [
            ("__table_base", self.table_base),
            ("__memory_base", self.memory_base),
        ] {
            let descriptor = Object::new();
            Reflect::set(&descriptor, &"value".into(), &"i32".into()).expect("setting descriptor");
            Reflect::set(&descriptor, &"mutable".into(), &false.into())
                .expect("setting descriptor2");

            // convert to i32 as the global is i32 in wasm
            let value_i32 = value as i32;

            let value =
                WebAssembly::Global::new(&descriptor, &value_i32.into()).expect("new global");
            Reflect::set(&env, &name.into(), &value.into()).expect("setting env global");
        }

        // Set the memory and table in the imports
        let imports = Object::new();
        Reflect::set(&imports, &"env".into(), &env).expect("setting env into imports");

        let instance = JsFuture::from(WebAssembly::instantiate_module(wasm_module, &imports))
            .await
            .expect("instantiating module");

        console::log_2(&"result instance".into(), &instance);

        let exports: Object = Reflect::get(&instance, &"exports".into())
            .expect("getting exports")
            .unchecked_into();

        // https://github.com/WebAssembly/tool-conventions/blob/main/DynamicLinking.md#relocations
        _ = Reflect::get(&exports, &"__wasm_apply_data_relocs".into())
            .unwrap()
            .unchecked_into::<js_sys::Function>()
            .call0(&JsValue::undefined());
        _ = Reflect::get(&exports, &"__wasm_apply_global_relocs".into())
            .unwrap()
            .unchecked_into::<js_sys::Function>()
            .call0(&JsValue::undefined());

        // https://github.com/WebAssembly/tool-conventions/blob/main/Linking.md#start-section
        _ = Reflect::get(&exports, &"__wasm_call_ctors".into())
            .unwrap()
            .unchecked_into::<js_sys::Function>()
            .call0(&JsValue::undefined())
    }
}

async fn load_wasm_module(jump_table: &JumpTable) -> Module {
    let path = jump_table.lib.to_str().unwrap();

    web_sys::console::info_1(&format!("Going to load wasm binary: {:?}", path).into());

    if !path.ends_with(".wasm") {
        panic!("The binary path in hotpatch message doesn't end with .wasm");
    }

    // fetch the module. use `fetch()` which exists both in main thread and web workers
    let global = js_sys::global();
    let response: Promise = if let Ok(window) = global.clone().dyn_into::<Window>() {
        window.fetch_with_str(&path)
    } else if let Ok(worker_global_scope) = global.dyn_into::<WorkerGlobalScope>() {
        worker_global_scope.fetch_with_str(&path)
    } else {
        panic!("globalThis is neither Window or WorkerGlobalScope")
    };

    // use compileStreaming instead of compile to enable caching https://v8.dev/blog/wasm-code-caching
    let module_promise = WebAssembly::compile_streaming(&response);

    let module: Module = JsFuture::from(module_promise)
        .await
        .expect("WebAssembly.compileStreaming error")
        .into();

    module
}

pub struct DylinkMemInfo {
    memory_size: u32,
    memory_alignment: u32,
    table_size: u32,
    table_alignment: u32,
}

pub struct DylinkSectionInfo {
    mem_info: DylinkMemInfo,
}

fn read_u8(buf: &mut &[u8]) -> Result<u8, PatchError> {
    let mut local = [0u8];
    match buf.read_exact(&mut local) {
        Ok(_) => {}
        Err(_) => {
            return Err(PatchError::WasmRelated(
                "Wasm dylink.0 section malformed (in read_u8)".to_string(),
            ));
        }
    }
    Ok(local[0])
}

fn read_leb_128_unsigned(buf: &mut &[u8]) -> Result<u64, PatchError> {
    match leb128::read::unsigned(buf) {
        Ok(v) => Ok(v),
        Err(e) => Err(PatchError::WasmRelated(
            "Wasm dylink.0 section malformed (in read_leb_128_unsigned)".to_string(),
        )),
    }
}

fn parse_dylink_section(module: &Module) -> Result<DylinkSectionInfo, PatchError> {
    let dylink_section_arr = WebAssembly::Module::custom_sections(&module, "dylink.0");
    if dylink_section_arr.length() == 0 {
        return Err(WasmRelated(
            "The hotpatch WASM binary doesn't have dylink.0 custom section".to_string(),
        ));
    }
    let dylink_section: ArrayBuffer = dylink_section_arr.get(0).into();
    let dylink_section = Uint8Array::new(&dylink_section);
    let mut dylink_bytes = vec![0u8; dylink_section.length() as usize];
    dylink_section.copy_to(&mut dylink_bytes);

    let mut buf: &[u8] = &dylink_bytes;

    let mut memory_info: Option<DylinkMemInfo> = None;
    loop {
        if buf.len() == 0 {
            break;
        }
        let sub_section_type = read_u8(&mut buf)?;
        let payload_len = read_leb_128_unsigned(&mut buf)? as usize;
        let mut sub_buf: &[u8] = &buf[0..payload_len];
        buf = &buf[payload_len..];
        match sub_section_type {
            1 => {
                memory_info = Some(DylinkMemInfo {
                    memory_size: read_leb_128_unsigned(&mut sub_buf)? as u32,
                    memory_alignment: read_leb_128_unsigned(&mut sub_buf)? as u32,
                    table_size: read_leb_128_unsigned(&mut sub_buf)? as u32,
                    table_alignment: read_leb_128_unsigned(&mut sub_buf)? as u32,
                });
            }
            _ => {}
        }

        console::log_1(&"Read one subsection in dylink.0".into())
    }

    Ok(DylinkSectionInfo {
        mem_info: match memory_info {
            None => {
                return Err(WasmRelated("No memory info in dylink.0".to_string()));
            }
            Some(v) => v,
        },
    })
}
