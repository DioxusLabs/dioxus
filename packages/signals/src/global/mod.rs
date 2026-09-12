use dioxus_core::{Runtime, ScopeId, Subscribers};
use generational_box::BorrowResult;
use std::{any::Any, cell::RefCell, collections::HashMap, ops::Deref, panic::Location, rc::Rc};

mod memo;
pub use memo::*;

mod signal;
pub use signal::*;

use crate::{Readable, ReadableExt, ReadableRef, Signal, Writable, WritableExt, WritableRef};

/// A trait for an item that can be constructed from an initialization function
pub trait InitializeFromFunction<T> {
    /// Create an instance of this type from an initialization function
    fn initialize_from_function(f: fn() -> T) -> Self;
}

impl<T> InitializeFromFunction<T> for T {
    fn initialize_from_function(f: fn() -> T) -> Self {
        f()
    }
}

/// A lazy value that is created once per application and can be accessed from anywhere in that application
pub struct Global<T, R = T> {
    constructor: fn() -> R,
    key: GlobalKey<'static>,
    normalized_file: std::sync::OnceLock<&'static str>,
    phantom: std::marker::PhantomData<fn() -> T>,
}

/// Allow calling a signal with signal() syntax
///
/// Currently only limited to copy types, though could probably specialize for string/arc/rc
impl<T: Clone, R: Clone> Deref for Global<T, R>
where
    T: Readable<Target = R> + InitializeFromFunction<R> + 'static,
    T::Target: 'static,
{
    type Target = dyn Fn() -> R;

    fn deref(&self) -> &Self::Target {
        unsafe { ReadableExt::deref_impl(self) }
    }
}

impl<T, R> Readable for Global<T, R>
where
    T: Readable<Target = R> + InitializeFromFunction<R> + Clone + 'static,
{
    type Target = R;
    type Storage = T::Storage;

    #[track_caller]
    fn try_read_unchecked(
        &self,
    ) -> Result<ReadableRef<'static, Self>, generational_box::BorrowError>
    where
        R: 'static,
    {
        self.resolve().try_read_unchecked()
    }

    #[track_caller]
    fn try_peek_unchecked(&self) -> BorrowResult<ReadableRef<'static, Self>>
    where
        R: 'static,
    {
        self.resolve().try_peek_unchecked()
    }

    fn subscribers(&self) -> Subscribers
    where
        R: 'static,
    {
        self.resolve().subscribers()
    }
}

impl<T: Clone, R> Writable for Global<T, R>
where
    T: Writable<Target = R> + InitializeFromFunction<R> + 'static,
{
    type WriteMetadata = T::WriteMetadata;

    #[track_caller]
    fn try_write_unchecked(
        &self,
    ) -> Result<WritableRef<'static, Self>, generational_box::BorrowMutError> {
        self.resolve().try_write_unchecked()
    }
}

impl<T: Clone, R> Global<T, R>
where
    T: Writable<Target = R> + InitializeFromFunction<R> + 'static,
{
    /// Write this value
    pub fn write(&self) -> WritableRef<'static, T, R> {
        self.resolve().try_write_unchecked().unwrap()
    }

    /// Run a closure with a mutable reference to the signal's value.
    /// If the signal has been dropped, this will panic.
    #[track_caller]
    pub fn with_mut<O>(&self, f: impl FnOnce(&mut R) -> O) -> O
    where
        T::Target: 'static,
    {
        self.resolve().with_mut(f)
    }
}

impl<T: Clone, R> Global<T, R>
where
    T: InitializeFromFunction<R>,
{
    #[track_caller]
    /// Create a new global value
    pub const fn new(constructor: fn() -> R) -> Self {
        let key = std::panic::Location::caller();
        Self {
            constructor,
            key: GlobalKey::new(key),
            normalized_file: std::sync::OnceLock::new(),
            phantom: std::marker::PhantomData,
        }
    }

    /// Create this global signal with a specific key.
    /// This is useful for ensuring that the signal is unique across the application and accessible from
    /// outside the application too.
    #[track_caller]
    pub const fn with_name(constructor: fn() -> R, key: &'static str) -> Self {
        Self {
            constructor,
            key: GlobalKey::File {
                file: key,
                line: 0,
                column: 0,
                index: 0,
            },
            normalized_file: std::sync::OnceLock::new(),
            phantom: std::marker::PhantomData,
        }
    }

    /// Create this global signal with a specific key.
    /// This is useful for ensuring that the signal is unique across the application and accessible from
    /// outside the application too.
    #[track_caller]
    pub const fn with_location(
        constructor: fn() -> R,
        file: &'static str,
        line: u32,
        column: u32,
        index: usize,
    ) -> Self {
        Self {
            constructor,
            key: GlobalKey::File {
                file,
                line: line as _,
                column: column as _,
                index: index as _,
            },
            normalized_file: std::sync::OnceLock::new(),
            phantom: std::marker::PhantomData,
        }
    }

    /// The key with its file path normalized to forward slashes, matching the compile-time
    /// `const_format` normalization the CLI applies (`file!()` may contain backslashes on
    /// Windows). The normalized path is computed at most once per call site and only when the
    /// raw path actually contains a backslash.
    fn normalized_key(&self) -> GlobalKey<'static> {
        let GlobalKey::File {
            file,
            line,
            column,
            index,
        } = &self.key
        else {
            return self.key.clone();
        };
        let (file, line, column, index) = (*file, *line, *column, *index);
        if !file.contains('\\') {
            return self.key.clone();
        }
        // Match `str_replace!(file!(), "\\\\", "/")` then `str_replace!(PATH, '\\', "/")`:
        // escaped backslashes are collapsed first, then single backslashes.
        let file = *self.normalized_file.get_or_init(|| {
            Box::leak(
                file.replace("\\\\", "/")
                    .replace('\\', "/")
                    .into_boxed_str(),
            )
        });
        GlobalKey::File {
            file,
            line,
            column,
            index,
        }
    }

    /// Get the key for this global
    pub fn key(&self) -> GlobalKey<'static> {
        self.normalized_key()
    }

    /// Resolve the global value. This will try to get the existing value from the current virtual dom, and if it doesn't exist, it will create a new one.
    // NOTE: This is not called "get" or "value" because those methods overlap with Readable and Writable
    pub fn resolve(&self) -> T
    where
        T: 'static,
    {
        let key = self.key();

        let context = get_global_context();

        // Get the entry if it already exists
        let mut evicted_stale_entry = false;
        {
            let read = context.map.borrow();
            if let Some(signal) = read.get(&key) {
                if let Some(signal) = signal.downcast_ref::<T>() {
                    return signal.clone();
                }
                evicted_stale_entry = true;
            }
        }

        if evicted_stale_entry {
            context.map.borrow_mut().remove(&key);
        }
        // Otherwise, create it
        // Constructors are always run in the root scope
        let signal = dioxus_core::Runtime::current().in_scope(ScopeId::ROOT, || {
            T::initialize_from_function(self.constructor)
        });
        context
            .map
            .borrow_mut()
            .insert(key, Box::new(signal.clone()));
        signal
    }

    /// Get the scope the signal was created in.
    pub fn origin_scope(&self) -> ScopeId {
        ScopeId::ROOT
    }
}

/// The context for global signals
#[derive(Clone, Default)]
pub struct GlobalLazyContext {
    map: Rc<RefCell<HashMap<GlobalKey<'static>, Box<dyn Any>>>>,
}

/// A key used to identify a signal in the global signal context
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum GlobalKey<'a> {
    /// A key derived from a `std::panic::Location` type
    File {
        /// The file name
        file: &'a str,

        /// The line number
        line: u32,

        /// The column number
        column: u32,

        /// The index of the signal in the file - used to disambiguate macro calls
        index: u32,
    },

    /// A raw key derived just from a string
    Raw(&'a str),
}

impl<'a> GlobalKey<'a> {
    /// Create a new key from a location
    pub const fn new(key: &'a Location<'a>) -> Self {
        GlobalKey::File {
            file: key.file(),
            line: key.line(),
            column: key.column(),
            index: 0,
        }
    }
}

impl From<&'static Location<'static>> for GlobalKey<'static> {
    fn from(key: &'static Location<'static>) -> Self {
        Self::new(key)
    }
}

impl GlobalLazyContext {
    /// Get a signal with the given string key
    /// The key will be converted to a UUID with the appropriate internal namespace
    pub fn get_signal_with_key<T: 'static>(&self, key: GlobalKey) -> Option<Signal<T>> {
        self.map.borrow().get(&key).map(|f| {
            *f.downcast_ref::<Signal<T>>().unwrap_or_else(|| {
                panic!(
                    "Global signal with key {:?} is not of the expected type. Keys are {:?}",
                    key,
                    self.map.borrow().keys()
                )
            })
        })
    }

    #[doc(hidden)]
    /// Clear all global signals of a given type.
    pub fn clear<T: 'static>(&self) {
        self.map.borrow_mut().retain(|_k, v| !v.is::<T>());
    }
}

/// The hot-reload template slot every `rsx!` site reads in debug builds.
#[doc(hidden)]
pub type HotReloadTemplateSignal = GlobalSignal<Option<dioxus_core::internal::HotReloadedTemplate>>;

/// Read guard over a [`HotReloadTemplateSignal`].
#[doc(hidden)]
pub type HotReloadTemplateRead = ReadableRef<'static, HotReloadTemplateSignal>;

/// Read an `rsx!` site's hot-reload slot, if a runtime is active.
#[doc(hidden)]
pub fn read_hot_reload_template(
    signal: &'static HotReloadTemplateSignal,
) -> Option<HotReloadTemplateRead> {
    Runtime::try_current().map(|_| signal.read())
}

/// Borrow the hot-reloaded template out of a [`read_hot_reload_template`] result.
#[doc(hidden)]
pub fn hot_reload_template(
    read: &Option<HotReloadTemplateRead>,
) -> Option<&dioxus_core::internal::HotReloadedTemplate> {
    read.as_ref().and_then(|read| read.as_ref())
}

/// The debug-build `static` state of one `rsx!` site: its hot-reload slot, the lazily lowered
/// runtime [`Template`](dioxus_core::Template), and the original-template metadata hot reload
/// falls back to. `rsx!` emits exactly one of these per site so a site costs a single item.
#[doc(hidden)]
pub struct RsxSite {
    /// The hot-reload template slot every render of the site reads.
    pub signal: HotReloadTemplateSignal,
    /// The runtime template, lowered from the view type on first render.
    pub template: std::sync::OnceLock<dioxus_core::Template>,
    /// The site's original-template metadata.
    pub meta: dioxus_core::internal::HotReloadSiteMeta,
}

impl RsxSite {
    /// Create the site state for the `rsx!` call at `file:line:column`, template `index`.
    #[doc(hidden)]
    pub const fn new(
        file: &'static str,
        line: u32,
        column: u32,
        index: usize,
        meta: dioxus_core::internal::HotReloadSiteMeta,
    ) -> Self {
        Self {
            signal: GlobalSignal::with_location(no_hot_reload_template, file, line, column, index),
            template: std::sync::OnceLock::new(),
            meta,
        }
    }
}

/// Initial value of every [`RsxSite`] hot-reload slot: one shared function rather than a closure
/// per site.
fn no_hot_reload_template() -> Option<dioxus_core::internal::HotReloadedTemplate> {
    None
}

/// The debug-build hot-reload state of one `rsx!` site for one render: the site's hot-reload
/// slot read plus the dynamic text pool its literals are rendered from.
///
/// `rsx!` creates this before evaluating the view (component literal props read through it) and
/// hands the finished [`VNode`](dioxus_core::VNode) back to [`HotReloadSite::finish`].
#[doc(hidden)]
pub struct HotReloadSite {
    site: &'static RsxSite,
    read: Option<HotReloadTemplateRead>,
    literal_pool: dioxus_core::internal::DynamicLiteralPool,
}

impl HotReloadSite {
    /// Read the site's hot-reload slot and build its literal pool from `dynamic_text`.
    #[doc(hidden)]
    pub fn new(site: &'static RsxSite, dynamic_text: Vec<String>) -> Self {
        Self {
            site,
            read: read_hot_reload_template(&site.signal),
            literal_pool: dioxus_core::internal::DynamicLiteralPool::new(dynamic_text),
        }
    }

    /// Get component literal `id`, falling back to `value` when the site has not been hot reloaded.
    #[doc(hidden)]
    pub fn component_property_or<T: 'static>(&self, id: usize, value: T) -> T {
        match hot_reload_template(&self.read) {
            Some(template) => self.literal_pool.component_property(id, template, value),
            None => value,
        }
    }

    /// Render `vnode` through the hot-reload pools, falling back to the site's original template.
    #[doc(hidden)]
    pub fn finish(self, vnode: dioxus_core::VNode) -> dioxus_core::VNode {
        dioxus_core::internal::render_hot_reloaded(
            vnode,
            hot_reload_template(&self.read),
            self.literal_pool,
            &self.site.meta,
        )
    }
}

/// Get the global context for signals
pub fn get_global_context() -> GlobalLazyContext {
    let rt = Runtime::current();
    match rt.has_context(ScopeId::ROOT) {
        Some(context) => context,
        None => rt.provide_context(ScopeId::ROOT, Default::default()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test that keys of global signals are correctly generated and different from one another.
    /// We don't want signals to merge, but we also want them to use both string IDs and memory addresses.
    // GlobalSignal contains a OnceLock for lazy file normalization, which trips the interior
    // mutability const lints; consts are intentional here (see the comment in the body).
    #[allow(
        clippy::declare_interior_mutable_const,
        clippy::borrow_interior_mutable_const
    )]
    #[test]
    fn test_global_keys() {
        // we're using consts since it's harder than statics due to merging - these won't be merged
        const MYSIGNAL: GlobalSignal<i32> = GlobalSignal::new(|| 42);
        const MYSIGNAL2: GlobalSignal<i32> = GlobalSignal::new(|| 42);
        const MYSIGNAL3: GlobalSignal<i32> = GlobalSignal::with_name(|| 42, "custom-keyed");

        let a = MYSIGNAL.key();
        let b = MYSIGNAL.key();
        let c = MYSIGNAL.key();
        assert_eq!(a, b);
        assert_eq!(b, c);

        let d = MYSIGNAL2.key();
        assert_ne!(a, d);

        let e = MYSIGNAL3.key();
        assert_ne!(a, e);
    }

    /// The runtime file normalization must match the compile-time `const_format` normalization
    /// that was previously emitted per `rsx!` site: `\\` collapses to `/` first, then `\` to `/`.
    #[allow(
        clippy::declare_interior_mutable_const,
        clippy::borrow_interior_mutable_const
    )]
    #[test]
    fn test_normalized_file_key() {
        const WINDOWS: GlobalSignal<i32> =
            GlobalSignal::with_location(|| 0, "C:\\\\Users\\\\x\\\\src\\\\main.rs", 1, 2, 0);
        let GlobalKey::File { file, .. } = WINDOWS.key() else {
            panic!("expected a file key")
        };
        // `\\` -> `/` first, so `\\U` becomes `/U`, not `//U`.
        assert_eq!(file, "C:/Users/x/src/main.rs");

        const PLAIN: GlobalSignal<i32> = GlobalSignal::with_location(|| 0, "src/main.rs", 1, 2, 0);
        let GlobalKey::File { file, .. } = PLAIN.key() else {
            panic!("expected a file key")
        };
        assert_eq!(file, "src/main.rs");

        const SINGLE: GlobalSignal<i32> =
            GlobalSignal::with_location(|| 0, "C:\\Users\\x\\main.rs", 1, 2, 0);
        let GlobalKey::File { file, .. } = SINGLE.key() else {
            panic!("expected a file key")
        };
        assert_eq!(file, "C:/Users/x/main.rs");
    }
}
