#![cfg(feature = "experimental_wasm_multithreading_support")]
#![cfg(target_arch = "wasm32")]

//! Currently, Wasm multithreading in browser has one important limitations: Wasm function table cannot be shared.
//!
//! Background: Wasm has no native function pointer, only function references. Function reference cannot be directly put into linear memory. Table is array-like thing that can hold function references. The function pointers are actually an index corresponding to a function reference in table.
//!
//! Currently, only the Wasm linear memory (backed by SharedArrayBuffer) can be shared across threads. Other things including `WebAssembly.Instance` and tables cannot be shared. Each web worker separately initialize their own `WebAssembly.Instance` and tables.
//!
//! Hotpatching requires dynamic linking. Dynamic linking requires loading new Wasm binary, creating new instance, and putting new functions into table. In Wasm multi-threading, doing dynamic linking requires all web workers to cooperatively dynamic link into their own tables. This is more complex than in single-threaded Wasm.
//!
//! Also, the tables in all threads must be kept in-sync. Because the function pointers(indices) can be shared across threads. All threads must dynamic link same Wasm binaries in the same order.
//!
//! The global jump table only updates after all web workers have dynamically linked the new code. (If not, the web worker cannot execute function pointers of new function).
//!
//! The multithreaded dynamic linking is an async process now. If new hotpatch comes before current hotpatch finishes, it needs to be queued.
//!
//! It uses `BroadcastChannel` to pass message of hotpatching. The hotpatch can only be triggered in main thread. When it triggers, send a message to workers via `BroadcastChannel`
//!
//! It internally allocates thread id for tracking what threads hasn't dynamic linked. Once all threads dynamic linked, update global jump table then use another `BroadcastChannel` to notify main thread. The main thread will do remaining queued hotpatches.
//!
//! Two public APIs:
//! - `init_hotpatch_for_current_thread`. It needs to be called once in main thread on init, and called once in each web worker on init.
//! - `close_hotpatch_for_current_thread`. It needs to be called in web worker before terminating web worker.
//!
//! These two APIs are exported to JS.

use crate::wasm_multithreading::CurrHotpatchingState::{
    Idle, MainThreadDynamicLinking, WebWorkersDynamicLinking,
};
use crate::PatchError::WasmRelated;
use crate::{commit_patch, wasm_is_multi_threaded, PatchError};
use js_sys::WebAssembly::{Memory, Module, Table};
use js_sys::{ArrayBuffer, Object, Promise, Reflect, Uint8Array, WebAssembly};
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::io::Read;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock};
use subsecond_types::JumpTable;
use wasm_bindgen::closure::Closure;
use wasm_bindgen::prelude::wasm_bindgen;
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;
use web_sys::{console, BroadcastChannel, MessageEvent, Window, WorkerGlobalScope};

/// It will set up `BroadcastChannel`s for hotpatching communication.
///
/// It should be called in main thread initialization. It should also be called in web worker initialization.
///
/// Note: dynamic linking happens in worker's event loop. The worker cannot process one message for too long time.
/// (If worker keeps running a scheduler loop, it cannot run `BroadcastChannel` callback)
#[wasm_bindgen]
pub async fn init_hotpatch_for_current_thread() {
    if !cfg!(debug_assertions) {
        return;
    }

    inner_init_hotpatch_for_current_thread().await;
}

/// It should be called in web worker before closing.
///
/// Note: if a web worker that initialized hotpatching is directly terminated without calling this,
/// next hotpatch will hang. Because hotpatching
#[wasm_bindgen]
pub fn close_hotpatch_for_current_thread() {
    if !cfg!(debug_assertions) {
        return;
    }

    inner_close_hotpatch_for_current_thread();
}

async fn inner_init_hotpatch_for_current_thread() {
    let old_state = CURR_THREAD_HOTPATCH_INIT_STATE.get();

    match old_state {
        CurrThreadHotpatchInitState::Uninitialized => {}
        CurrThreadHotpatchInitState::Initialized => {
            console::debug_1(
                &format!(
                    "[subsecond] Current thread {:?} has already initialized hotpatch",
                    get_my_thread_id()
                )
                .into(),
            );
        }
        CurrThreadHotpatchInitState::Closed => {
            console::error_1(
                &format!(
                    "[subsecond] Current thread {:?} is already in closed state, cannot init",
                    get_my_thread_id()
                )
                .into(),
            );
            return;
        }
    }

    CURR_THREAD_HOTPATCH_INIT_STATE.set(CurrThreadHotpatchInitState::Initialized);

    assert!(
        wasm_is_multi_threaded(),
        "init_hotpatch_for_current_thread can only be used in multi-threading"
    );

    console::debug_1(&format!("[subsecond] Thread {:?} initializing", get_my_thread_id()).into());

    let mut global_hotpatch_state = GLOBAL_HOTPATCH_STATE.lock();

    // The BroadcastChannel starts receiving message upon creation,
    // so it needs to be created under lock
    let to_hotpatch_channel: BroadcastChannel =
        BroadcastChannel::new(CHANNEL_WORKER_SHOULD_DYNAMIC_LINK).expect("creating channel 1");

    let hotpatch_finish_channel: BroadcastChannel =
        BroadcastChannel::new(CHANNEL_WORKER_DYNAMIC_LINKED).expect("creating channel 2");

    let mut to_hotpatch_callback: Option<JsValue> = None;
    let mut hotpatch_finish_callback: Option<JsValue> = None;

    if is_main_thread() {
        let closure: Closure<dyn Fn(&MessageEvent)> =
            Closure::new(move |e: &MessageEvent| on_main_thread_know_hotpatch_finish());
        let closure_js = closure.into_js_value();
        hotpatch_finish_callback = Some(closure_js.clone());
        hotpatch_finish_channel.set_onmessage(Some(&closure_js.into()));
    } else {
        let closure: Closure<dyn Fn(&MessageEvent)> =
            Closure::new(move |e: &MessageEvent| on_worker_should_dynamic_link(e));
        let closure_js = closure.into_js_value();
        to_hotpatch_callback = Some(closure_js.clone());
        to_hotpatch_channel.set_onmessage(Some(&closure_js.into()));
    }

    CHANNEL_LOCAL_STATE.with(|r| {
        *r.borrow_mut() = Some(ChannelThreadLocalState {
            hotpatch_finish_channel: hotpatch_finish_channel.clone(),
            hotpatch_finish_callback: hotpatch_finish_callback.clone(),
            to_hotpatch_channel: to_hotpatch_channel.clone(),
            to_hotpatch_callback: to_hotpatch_callback.clone(),
        });
    });

    let already_hotpatched: Vec<Arc<HotpatchEntry>> = global_hotpatch_state.hotpatched.clone();

    if !is_main_thread() {
        global_hotpatch_state
            .worker_thread_ids
            .insert(get_my_thread_id());

        match global_hotpatch_state.curr_state {
            WebWorkersDynamicLinking(ref mut web_worker_dynamic_linking_state) => {
                console::debug_1(
                    &format!(
                        "Web worker {:?} initializes during a pending hotpatch.",
                        get_my_thread_id()
                    )
                    .into(),
                );
                web_worker_dynamic_linking_state
                    .pending_thread_ids
                    .insert(get_my_thread_id());
            }
            _ => {}
        }
    }

    // unlock
    drop(global_hotpatch_state);

    let already_patch_count = already_hotpatched.len();

    if already_patch_count != 0 {
        console::debug_1(
            &format!(
                "Web worker {:?} is going to dynamic-link {} existing hotpatches.",
                get_my_thread_id(),
                already_patch_count
            )
            .into(),
        );

        // the new web worker needs to dynamic-link the existing hotpatches before its launch
        for entry in already_hotpatched {
            let module = load_wasm_module(&entry.jump_table).await;

            entry.internal_per_thread_dynamic_link(&module).await;
        }
    }
}

fn inner_close_hotpatch_for_current_thread() {
    assert!(!is_main_thread(), "Cannot be called in main thread");

    let current_state = CURR_THREAD_HOTPATCH_INIT_STATE.get();
    if current_state == CurrThreadHotpatchInitState::Closed {
        console::debug_1(
            &format!(
                "[subsecond] Current web worker {:?} has already closed hotpatch",
                get_my_thread_id()
            )
            .into(),
        );
        return;
    }

    if current_state == CurrThreadHotpatchInitState::Uninitialized {
        console::warn_1(
            &format!(
                "[subsecond] Current web worker {:?} is closing hotpatch without initializing first",
                get_my_thread_id()
            )
            .into(),
        );
        return;
    }

    CHANNEL_LOCAL_STATE.with(|r| {
        let mut state = r.borrow_mut();
        if let Some(channel_state) = &mut *state {
            channel_state.to_hotpatch_channel.set_onmessage(None);
            channel_state.hotpatch_finish_channel.set_onmessage(None);
        }
        *state = None;
    });

    // Remove thread ID from global state (only for web workers, not main thread)
    if !is_main_thread() {
        let mut global_state = GLOBAL_HOTPATCH_STATE.lock();
        global_state.worker_thread_ids.remove(&get_my_thread_id());

        // Also remove from pending_thread_ids if currently in WebWorkersDynamicLinking state
        if let CurrHotpatchingState::WebWorkersDynamicLinking(dynamic_linking_state) =
            &mut global_state.curr_state
        {
            dynamic_linking_state
                .pending_thread_ids
                .remove(&get_my_thread_id());

            if dynamic_linking_state.pending_thread_ids.is_empty() {
                drop(global_state);
                console::debug_1(
                    &"[subsecond] All web workers finished hotpatching (triggered on web worker close)".into(),
                );
                notify_main_thread_hotpatch_finish();
            }
        }
    }

    CURR_THREAD_HOTPATCH_INIT_STATE.set(CurrThreadHotpatchInitState::Closed);

    console::debug_1(
        &format!(
            "[subsecond] Thread {:?} closed hotpatch",
            get_my_thread_id()
        )
        .into(),
    );
}

static NEXT_THREAD_ID: AtomicUsize = AtomicUsize::new(0);

#[derive(Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Debug)]
struct MyThreadId(usize);

thread_local! {
    static IS_MAIN_THREAD: bool = web_sys::window().is_some();

    /// This thread id is for internally tracking what web worker haven't dynamically linked the patch
    static MY_THREAD_ID: MyThreadId = MyThreadId(NEXT_THREAD_ID.fetch_add(1, Ordering::Relaxed));

    static CURR_THREAD_HOTPATCH_INIT_STATE: Cell<CurrThreadHotpatchInitState> =
        Cell::new(CurrThreadHotpatchInitState::Uninitialized);
}

fn get_my_thread_id() -> MyThreadId {
    MY_THREAD_ID.with(|s| *s)
}

fn is_main_thread() -> bool {
    IS_MAIN_THREAD.with(|s| *s)
}

struct HotpatchEntry {
    jump_table: JumpTable,
    table_base: u64,
    memory_base: u64,
}

enum CurrHotpatchingState {
    Idle,
    MainThreadDynamicLinking,
    WebWorkersDynamicLinking(WebWorkersDynamicLinkingState),
}

#[derive(Copy, Clone, Hash, Ord, PartialOrd, Eq, PartialEq, Debug)]
enum CurrThreadHotpatchInitState {
    Uninitialized,
    Initialized,
    Closed,
}

struct WebWorkersDynamicLinkingState {
    hotpatch_entry: Arc<HotpatchEntry>,
    pending_thread_ids: HashSet<MyThreadId>,
}

struct GlobalHotpatchState {
    /// The hotpatches that have already been done.
    /// They will be re-dynamic-linked for each new web worker
    hotpatched: Vec<Arc<HotpatchEntry>>,
    /// The hotpatch that's being done (waiting for web workers to dynamically link)
    curr_state: CurrHotpatchingState,
    /// When a new hotpatch comes before current hotpatch finishes, it's queued
    pending_hotpatches: Vec<JumpTable>,
    /// Collect all web worker thread ids. Doesn't contain main threads'.
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
pub(crate) async unsafe fn wasm_multithreaded_hotpatch_trigger(jump_table: JumpTable) {
    {
        let mut hotpatch_state = GLOBAL_HOTPATCH_STATE.lock();
        match hotpatch_state.curr_state {
            Idle => {
                hotpatch_state.curr_state = MainThreadDynamicLinking;
            }
            _ => {
                console::debug_1(
                    &"[subsecond] Received new hotpatch when previous hotpatch hasn't finished. Queue it."
                        .into(),
                );
                hotpatch_state.pending_hotpatches.push(jump_table);
                return;
                // About why not use async lock: futures crate's async lock has no order guarantee:
                // https://docs.rs/futures/0.3.31/futures/lock/struct.Mutex.html#fairness
                // Use manual queueing to ensure new patch won't be overwritten by old patch
            }
        }
    }

    let entry = main_thread_prepare_and_hotpatch(jump_table).await;

    {
        let mut hotpatch_state = GLOBAL_HOTPATCH_STATE.lock();

        assert!(
            matches!(hotpatch_state.curr_state, MainThreadDynamicLinking),
            "curr_state is not MainThreadDynamicLinking"
        );

        if hotpatch_state.worker_thread_ids.is_empty() {
            console::debug_1(&"No web worker, directly finish hotpatch".into());
            on_main_thread_know_hotpatch_finish();
        } else {
            hotpatch_state.curr_state = WebWorkersDynamicLinking(WebWorkersDynamicLinkingState {
                hotpatch_entry: Arc::new(entry),
                pending_thread_ids: hotpatch_state.worker_thread_ids.clone(),
            });

            notify_web_workers_to_dynamic_link();
        }
    }
}

async fn main_thread_prepare_and_hotpatch(mut jump_table: JumpTable) -> HotpatchEntry {
    assert!(is_main_thread());
    assert!(
        CURR_THREAD_HOTPATCH_INIT_STATE.get() == CurrThreadHotpatchInitState::Initialized,
        "main thread hasn't called init_hotpatch_for_current_thread"
    );

    let funcs: Table = wasm_bindgen::function_table().unchecked_into();
    let table_base = funcs.length();

    // the function addresses are relative. add them with table base to become absolute
    // in Wasm, function address means offset into function table
    for v in jump_table.map.values_mut() {
        *v += table_base as u64;
    }

    let module = load_wasm_module(&mut jump_table).await;

    let dylink_section_info = parse_dylink_section(&module).expect("Cannot parse dylink.0 section");

    console::debug_1(
        &format!(
            "[subsecond] The patch's required data size {}",
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

    entry
}

/// sent from main thread to workers, to tell them to dynamic link
static CHANNEL_WORKER_SHOULD_DYNAMIC_LINK: &str = "__subsecond_worker_should_dynamic_link";

/// sent from worker to main thread, to tell main thread that a worker has dynamically linked
static CHANNEL_WORKER_DYNAMIC_LINKED: &str = "__subsecond_worker_dynamic_linked";

struct ChannelThreadLocalState {
    /// When main thread starts a new hotpatch, send a message to this channel
    to_hotpatch_channel: BroadcastChannel,
    to_hotpatch_callback: Option<JsValue>,
    /// When the last pending worker finishes dynaic link, send a message to this channel
    hotpatch_finish_channel: BroadcastChannel,
    hotpatch_finish_callback: Option<JsValue>,
    // Why two `BroadcastChannel`s instead of one: the finishing message should be processed by only main thread. using one `BroadcastChannel`` will make all workers process that message needlessly.
}

thread_local! {
    static CHANNEL_LOCAL_STATE: RefCell<Option<ChannelThreadLocalState>> = RefCell::new(None);
}

fn notify_web_workers_to_dynamic_link() {
    assert!(is_main_thread());

    CHANNEL_LOCAL_STATE.with(|s| {
        let borrow = s.borrow();
        let s = borrow
            .as_ref()
            .expect("channel local state not initialized");

        // message content doesn't matter
        s.to_hotpatch_channel
            .post_message(&"worker should dynamic link".into())
            .expect("send failed");
    })
}

fn on_main_thread_know_hotpatch_finish() {
    assert!(is_main_thread());

    let mut state = GLOBAL_HOTPATCH_STATE.lock();

    let web_worker_dynamic_linking_state = match state.curr_state {
        WebWorkersDynamicLinking(ref web_workers_dynamic_linking_state) => {
            web_workers_dynamic_linking_state
        }
        _ => {
            panic!("on_main_thread_receive_hotpatch_finish in wrong state")
        }
    };

    assert!(web_worker_dynamic_linking_state
        .pending_thread_ids
        .is_empty());

    unsafe {
        web_worker_dynamic_linking_state
            .hotpatch_entry
            .apply_change_to_jump_table();
    }

    state.curr_state = Idle;

    if !state.pending_hotpatches.is_empty() {
        // transfer state to next hotpatch when holding lock
        let next_to_patch = state.pending_hotpatches.remove(0);

        state.curr_state = MainThreadDynamicLinking;

        wasm_bindgen_futures::spawn_local(async move {
            main_thread_prepare_and_hotpatch(next_to_patch);
        });
    }
}

fn notify_main_thread_hotpatch_finish() {
    CHANNEL_LOCAL_STATE.with(|s| {
        let borrowed = s.borrow();

        borrowed
            .as_ref()
            .expect("channel not initialized")
            .hotpatch_finish_channel
            .post_message(&"hotpatch finished in web workers".into())
            .expect("send failed");
    })
}

fn on_worker_should_dynamic_link(event: &MessageEvent) {
    assert!(!is_main_thread());

    let state = GLOBAL_HOTPATCH_STATE.lock();

    let entry = match &state.curr_state {
        WebWorkersDynamicLinking(web_workers_dynamic_linking_state) => {
            web_workers_dynamic_linking_state.hotpatch_entry.clone()
        }
        _ => {
            panic!("Wrong state in on_worker_should_dynamic_link")
        }
    };

    // unlock
    drop(state);

    wasm_bindgen_futures::spawn_local(async move {
        let wasm_module = load_wasm_module(&entry.jump_table).await;

        entry.internal_per_thread_dynamic_link(&wasm_module).await;

        let mut state = GLOBAL_HOTPATCH_STATE.lock();

        let finished = match &mut state.curr_state {
            WebWorkersDynamicLinking(ref mut web_workers_dynamic_linking_state) => {
                let my_thread_id = get_my_thread_id();
                let removed = web_workers_dynamic_linking_state
                    .pending_thread_ids
                    .remove(&my_thread_id);

                if !removed {
                    console::error_1(
                        &format!(
                            "[subsecond] Current web worker not in pending_thread_ids {:?}",
                            my_thread_id
                        )
                        .into(),
                    );
                }

                console::debug_1(
                    &format!(
                        "[subsecond] Web worker {:?} finished dynamic linking",
                        my_thread_id
                    )
                    .into(),
                );

                web_workers_dynamic_linking_state
                    .pending_thread_ids
                    .is_empty()
            }
            _ => {
                panic!("Wrong state in on_worker_should_dynamic_link after dynamic link")
            }
        };

        if finished {
            console::debug_1(&"[subsecond] All web workers finished hotpatching".into());
            notify_main_thread_hotpatch_finish();
        }
    });
}

impl HotpatchEntry {
    unsafe fn apply_change_to_jump_table(&self) {
        unsafe { commit_patch(self.jump_table.clone()) };
    }

    async fn internal_per_thread_dynamic_link(&self, wasm_module: &Module) {
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

        console::debug_2(&"[subsecond] result instance".into(), &instance);

        let exports: Object = Reflect::get(&instance, &"exports".into())
            .expect("getting exports")
            .unchecked_into();

        // https://github.com/WebAssembly/tool-conventions/blob/main/DynamicLinking.md#relocations
        _ = Reflect::get(&exports, &"__wasm_apply_data_relocs".into())
            .unwrap()
            .unchecked_into::<js_sys::Function>()
            .call0(&JsValue::undefined());

        // in my testing, there is no __wasm_apply_global_relocs or __wasm_call_ctors, no need to call them

        // initialize patch binary's __tls_base to be same as parent's
        let patch_tls_base_global = Reflect::get(&exports, &"__tls_base".into())
            .expect("getting __tls_base export in patch")
            .dyn_into::<WebAssembly::Global>()
            .expect("invalid __tls_base in patch export");

        let parent_exports = wasm_bindgen::exports();
        let parent_tls_base_global = Reflect::get(&parent_exports, &"__tls_base".into())
            .expect("getting __tls_base export in parent")
            .dyn_into::<WebAssembly::Global>()
            .expect("invalid __tls_base in parent export");

        let parent_tls_base_value = parent_tls_base_global.value();
        console::debug_2(&"[subsecond] __tls_base : ".into(), &parent_tls_base_value);
        patch_tls_base_global.set_value(&parent_tls_base_value);
    }
}

async fn load_wasm_module(jump_table: &JumpTable) -> Module {
    let path = jump_table.lib.to_str().unwrap();

    web_sys::console::debug_1(&format!("[subsecond] Going to load wasm binary: {:?}", path).into());

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

        console::debug_1(&"[subsecond] Read one subsection in dylink.0".into())
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
