use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    hash::Hash,
    marker::PhantomData,
    rc::Rc,
};

pub use atomfamily::*;
pub use atoms::*;
pub use ecs::*;
use error::*;
pub use hooks::*;
pub use hooks::*;
pub use root::*;
pub use selector::*;
pub use selectorfamily::*;
pub use traits::*;
pub use utils::*;
mod tracingimmap;

mod traits {
    use dioxus_core::prelude::Context;

    use super::*;
    pub trait MapKey: PartialEq + Hash + 'static {}
    impl<T: PartialEq + Hash + 'static> MapKey for T {}

    pub trait AtomValue: PartialEq + 'static {}
    impl<T: PartialEq + 'static> AtomValue for T {}

    // Atoms, selectors, and their family variants are readable
    pub trait Readable<T: AtomValue>: Sized + Copy {
        fn use_read<'a, P: 'static>(self, ctx: Context<'a, P>) -> &'a T {
            hooks::use_read(ctx, self)
        }

        // This returns a future of the value
        // If the atom is currently pending, that future will resolve to pending
        // If the atom is currently ready, the future will immediately resolve
        // if the atom switches from ready to pending, the component will re-run, returning a pending future
        fn use_read_async<'a, P>(self, ctx: Context<'a, P>) -> &'a T {
            todo!()
        }

        fn initialize(self, api: &RecoilRoot) -> T;

        // We use the Raw Ptr to the atom
        // TODO: Make sure atoms with the same definitions don't get merged together. I don't think they do, but double check
        fn static_id(self) -> u32;
    }

    // Only atoms and atom families are writable
    // Selectors and selector families are not
    pub trait Writable<T: AtomValue>: Readable<T> + Sized {
        fn use_read_write<'a, P>(self, ctx: Context<'a, P>) -> (&'a T, &'a Rc<dyn Fn(T)>) {
            todo!()
        }

        fn use_write<'a, P>(self, ctx: Context<'a, P>) -> &'a Rc<dyn Fn(T)> {
            todo!()
        }
    }
}

mod atoms {
    use super::*;

    // Currently doesn't do anything, but will eventually add effects, id, serialize/deserialize keys, etc
    // Doesn't currently allow async values, but the goal is to eventually enable them
    pub struct AtomBuilder {}

    pub type Atom<T> = fn(&mut AtomBuilder) -> T;

    // impl<T: AtomValue> Readable<T> for Atom<T> {}
    impl<T: AtomValue> Readable<T> for &'static Atom<T> {
        fn static_id(self) -> u32 {
            self as *const _ as u32
        }

        fn initialize(self, api: &RecoilRoot) -> T {
            let mut builder = AtomBuilder {};
            let p = self(&mut builder);
            p
        }
    }

    impl<T: AtomValue> Writable<T> for &'static Atom<T> {}

    mod compilests {
        use super::*;
        use dioxus_core::prelude::Context;

        fn _test(ctx: Context<()>) {
            const EXAMPLE_ATOM: Atom<i32> = |_| 10;

            // ensure that atoms are both read and write
            let _ = use_read(ctx, &EXAMPLE_ATOM);
            let _ = use_read_write(ctx, &EXAMPLE_ATOM);
            let _ = use_write(ctx, &EXAMPLE_ATOM);
        }
    }
}

mod atomfamily {
    use super::*;
    pub trait FamilyCollection<K, V> {}
    impl<K, V> FamilyCollection<K, V> for HashMap<K, V> {}

    use im_rc::HashMap as ImHashMap;

    /// AtomHashMaps provide an efficient way of maintaing collections of atoms.
    ///
    /// Under the hood, AtomHashMaps uses [IM](https://www.rust-lang.org)'s immutable HashMap implementation to lazily
    /// clone data as it is modified.
    ///
    ///
    ///
    ///
    ///
    ///
    pub type AtomHashMap<K, V> = fn(&mut ImHashMap<K, V>);

    pub trait AtomFamilySelector<K: MapKey, V: AtomValue + Clone> {
        fn select(&'static self, k: &K) -> AtomMapSelection<K, V> {
            todo!()
        }
    }

    impl<K: MapKey, V: AtomValue + Clone> AtomFamilySelector<K, V> for AtomHashMap<K, V> {
        fn select(&'static self, k: &K) -> AtomMapSelection<K, V> {
            todo!()
        }
    }

    pub struct AtomMapSelection<'a, K: MapKey, V: AtomValue> {
        root: &'static AtomHashMap<K, V>,
        key: &'a K,
    }

    impl<'a, K: MapKey, V: AtomValue> Readable<V> for &AtomMapSelection<'a, K, V> {
        fn static_id(self) -> u32 {
            todo!()
        }

        fn initialize(self, api: &RecoilRoot) -> V {
            todo!()
            // let mut builder = AtomBuilder {};
            // let p = self(&mut builder);
            // p
        }
    }

    impl<'a, K: MapKey, T: AtomValue> Writable<T> for &AtomMapSelection<'a, K, T> {}

    mod compiletests {
        use dioxus_core::prelude::Context;

        use super::*;
        const Titles: AtomHashMap<u32, &str> = |map| {};

        fn test(ctx: Context<()>) {
            let title = Titles.select(&10).use_read(ctx);
            let t2 = use_read(ctx, &Titles.select(&10));
        }
    }
}

mod selector {
    use super::*;
    pub struct SelectorBuilder {}
    impl SelectorBuilder {
        pub fn get<T: AtomValue>(&self, t: impl Readable<T>) -> &T {
            todo!()
        }
    }
    pub type Selector<T> = fn(&mut SelectorBuilder) -> T;
    impl<T: AtomValue> Readable<T> for &'static Selector<T> {
        fn static_id(self) -> u32 {
            todo!()
        }

        fn initialize(self, api: &RecoilRoot) -> T {
            todo!()
        }
    }

    pub struct SelectorFamilyBuilder {}

    impl SelectorFamilyBuilder {
        pub fn get<T: AtomValue>(&self, t: impl Readable<T>) -> &T {
            todo!()
        }
    }
}
mod selectorfamily {
    use super::*;
    // pub trait SelectionSelector<K, V> {
    //     fn select(&self, k: &K) -> CollectionSelection<K, V> {
    //         todo!()
    //     }
    // }
    // impl<K, V, F> SelectionSelector<K, V> for AtomFamily<K, V, F> {}

    /// Create a new value as a result of a combination of previous values
    /// If you need to return borrowed data, check out [`SelectorFamilyBorrowed`]
    pub type SelectorFamily<Key, Value> = fn(&mut SelectorFamilyBuilder, Key) -> Value;

    impl<K, V: AtomValue> Readable<V> for &'static SelectorFamily<K, V> {
        fn static_id(self) -> u32 {
            todo!()
        }

        fn initialize(self, api: &RecoilRoot) -> V {
            todo!()
        }
    }

    /// Borrowed selector families are – surprisingly - discouraged.
    /// This is because it's not possible safely memoize these values without keeping old versions around.
    ///
    /// However, it does come in handy to borrow the contents of an item without re-rendering child components.
    pub type SelectorFamilyBorrowed<Key, Value> =
        for<'a> fn(&'a mut SelectorFamilyBuilder, Key) -> &'a Value;

    // impl<'a, K, V: 'a> SelectionSelector<K, V> for fn(&'a mut SelectorFamilyBuilder, K) -> V {}
}

mod root {
    use std::{
        any::{Any, TypeId},
        collections::{HashSet, VecDeque},
        iter::FromIterator,
        sync::atomic::{AtomicU32, AtomicUsize},
    };

    use super::*;
    // use generational_arena::Index as ConsumerId;
    type AtomId = u32;
    type ConsumerId = u32;

    pub type RecoilContext = RefCell<RecoilRoot>;

    // Sometimes memoization means we don't need to re-render components that holds "correct values"
    // IE we consider re-render more expensive than keeping the old value around.
    // We *could* unsafely overwrite this slot, but that's just **asking** for UB (holding a &mut while & is held in components)
    //
    // Instead, we choose to let the hook itself hold onto the Rc<T> by not forcing a render when T is the same.
    // Whenever the component needs to be re-rendered for other reasons, the "get" method will automatically update the Rc<T> to the most recent one.
    pub struct RecoilRoot {
        nodes: RefCell<HashMap<AtomId, Slot>>,
        consumer_map: HashMap<ConsumerId, AtomId>,
    }

    struct Slot {
        type_id: TypeId,
        source: AtomId,
        value: Rc<dyn Any>,
        consumers: HashMap<ConsumerId, Rc<dyn Fn()>>,
        dependents: HashSet<AtomId>,
    }

    static NEXT_ID: AtomicU32 = AtomicU32::new(0);
    fn next_consumer_id() -> u32 {
        NEXT_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    }

    impl RecoilRoot {
        pub(crate) fn new() -> Self {
            Self {
                nodes: Default::default(),
                consumer_map: Default::default(),
            }
        }

        pub fn subscribe<T: AtomValue>(
            &mut self,
            readable: impl Readable<T>,
            receiver_fn: Rc<dyn Fn()>,
        ) -> ConsumerId {
            let consumer_id = next_consumer_id();
            let atom_id = readable.static_id();
            log::debug!("Subscribing consumer to atom {} {}", consumer_id, atom_id);

            let mut nodes = self.nodes.borrow_mut();
            let slot = nodes.get_mut(&atom_id).unwrap();
            slot.consumers.insert(consumer_id, receiver_fn);
            self.consumer_map.insert(consumer_id, atom_id);
            consumer_id
        }

        pub fn unsubscribe(&mut self, consumer_id: ConsumerId) {
            let atom_id = self.consumer_map.get(&consumer_id).unwrap();
            let mut nodes = self.nodes.borrow_mut();
            let slot = nodes.get_mut(&atom_id).unwrap();
            slot.consumers.remove(&consumer_id);
        }

        /// Directly get the *slot*
        /// All Atoms are held in slots (an Rc)
        ///
        ///
        pub fn try_get_raw<T: AtomValue>(&self, readable: impl Readable<T>) -> Result<Rc<T>> {
            let atom_id = readable.static_id();
            let mut nodes = self.nodes.borrow_mut();
            if !nodes.contains_key(&atom_id) {
                let value = Slot {
                    type_id: TypeId::of::<T>(),
                    source: atom_id,
                    value: Rc::new(readable.initialize(self)),
                    consumers: Default::default(),
                    dependents: Default::default(),
                };
                nodes.insert(atom_id, value);
            }
            let out = nodes
                .get(&atom_id)
                .unwrap()
                .value
                .clone()
                .downcast::<T>()
                .unwrap();

            Ok(out)
        }

        pub fn try_set<T: AtomValue>(
            &mut self,
            writable: impl Writable<T>,
            new_val: T,
        ) -> crate::error::Result<()> {
            let atom_id = writable.static_id();

            self.set_by_id(atom_id, new_val);

            Ok(())
        }

        // A slightly dangerous method to manually overwrite any slot given an AtomId
        pub(crate) fn set_by_id<T: AtomValue>(&mut self, atom_id: AtomId, new_val: T) {
            let mut nodes = self.nodes.borrow_mut();
            let consumers = match nodes.get_mut(&atom_id) {
                Some(slot) => {
                    slot.value = Rc::new(new_val);
                    &slot.consumers
                }
                None => {
                    let value = Slot {
                        type_id: TypeId::of::<T>(),
                        source: atom_id,
                        value: Rc::new(new_val),
                        // value: Rc::new(writable.initialize(self)),
                        consumers: Default::default(),
                        dependents: Default::default(),
                    };
                    nodes.insert(atom_id, value);
                    &nodes.get(&atom_id).unwrap().consumers
                }
            };

            for (id, consumer_fn) in consumers {
                log::debug!("triggering selector {}", id);
                consumer_fn();
            }
        }
    }
}

mod hooks {
    use super::*;
    use dioxus_core::{hooks::use_ref, prelude::Context};

    pub fn use_init_recoil_root<P>(ctx: Context<P>, cfg: impl Fn(())) {
        ctx.use_create_context(move || RefCell::new(RecoilRoot::new()))
    }

    /// Gain access to the recoil API directly - set, get, modify, everything
    /// This is the foundational hook in which read/write/modify are built on
    ///
    /// This does not subscribe the component to *any* updates
    ///
    /// You can use this method to create controllers that perform much more complex actions than set/get
    /// However, be aware that "getting" values through this hook will not subscribe the component to any updates.
    pub fn use_recoil_api<'a, P>(ctx: Context<'a, P>) -> &Rc<RecoilContext> {
        ctx.use_context::<RecoilContext>()
    }

    pub fn use_write<'a, T: AtomValue, P>(
        ctx: Context<'a, P>,
        // todo: this shouldn't need to be static
        writable: impl Writable<T>,
    ) -> &'a Rc<dyn Fn(T)> {
        let api = use_recoil_api(ctx);
        ctx.use_hook(
            move || {
                let api = api.clone();
                let raw_id = writable.static_id();
                Rc::new(move |new_val| {
                    //
                    log::debug!("setting new value ");
                    let mut api = api.as_ref().borrow_mut();

                    // api.try_set(writable, new_val).expect("failed to set");
                    api.set_by_id(raw_id, new_val);
                }) as Rc<dyn Fn(T)>
            },
            move |hook| &*hook,
            |hook| {},
        )
    }

    /// Read the atom and get the Rc directly to the Atom's slot
    /// This is useful if you need the memoized Atom value. However, Rc<T> is not as easy to
    /// work with as
    pub fn use_read_raw<'a, T: AtomValue, P: 'static>(
        ctx: Context<'a, P>,
        readable: impl Readable<T>,
    ) -> &Rc<T> {
        struct ReadHook<T> {
            value: Rc<T>,
            consumer_id: u32,
        }

        let api = use_recoil_api(ctx);
        ctx.use_hook(
            move || {
                let mut api = api.as_ref().borrow_mut();

                let update = ctx.schedule_update();
                let val = api.try_get_raw(readable).unwrap();
                let id = api.subscribe(readable, update);
                ReadHook {
                    value: val,
                    consumer_id: id,
                }
            },
            move |hook| {
                let api = api.as_ref().borrow();

                let val = api.try_get_raw(readable).unwrap();
                hook.value = val;
                &hook.value
            },
            move |hook| {
                let mut api = api.as_ref().borrow_mut();
                api.unsubscribe(hook.consumer_id);
            },
        )
    }

    ///
    pub fn use_read<'a, T: AtomValue, P: 'static>(
        ctx: Context<'a, P>,
        readable: impl Readable<T>,
    ) -> &'a T {
        use_read_raw(ctx, readable).as_ref()
    }

    /// # Use an atom in both read and write
    ///
    /// This method is only available for atoms and family selections (not selectors).
    ///
    /// This is equivalent to calling both `use_read` and `use_write`, but saves you the hassle and repitition
    ///
    /// ## Example
    ///
    /// ```
    /// const Title: Atom<&str> = |_| "hello";
    /// let (title, set_title) = use_read_write(ctx, &Title);
    ///
    /// // equivalent to:
    /// let (title, set_title) = (use_read(ctx, &Title), use_write(ctx, &Title));
    /// ```
    pub fn use_read_write<'a, T: AtomValue + 'static, P: 'static>(
        ctx: Context<'a, P>,
        writable: impl Writable<T>,
    ) -> (&'a T, &'a Rc<dyn Fn(T)>) {
        (use_read(ctx, writable), use_write(ctx, writable))
    }

    /// # Modify an atom without using `use_read`.
    ///
    /// Occasionally, a component might want to write to an atom without subscribing to its changes. `use_write` does not
    /// provide this functionality, so `use_modify` exists to gain access to the current atom value while setting it.
    ///
    /// ## Notes
    ///
    /// Do note that this hook can only be used with Atoms where T: Clone since we actually clone the current atom to make
    /// it mutable.
    ///
    /// Also note that you need to stack-borrow the closure since the modify closure expects an &dyn Fn. If we made it
    /// a static type, it wouldn't be possible to use the `modify` closure more than once (two closures always are different)
    ///
    /// ## Example
    ///
    /// ```ignore
    /// let modify_atom = use_modify(ctx, Atom);
    ///
    /// modify_atom(&|a| *a += 1)
    /// ```
    pub fn use_modify<'a, T: AtomValue + 'static + Clone, P>(
        ctx: Context<'a, P>,
        writable: impl Writable<T>,
    ) -> impl Fn(&dyn Fn()) {
        |_| {}
    }

    /// Use a family collection directly
    /// !! Any changes to the family will cause this subscriber to update
    /// Try not to put this at the very top-level of your app.
    pub fn use_read_family<'a, K, V, P>(
        ctx: Context<'a, P>,
        t: &AtomHashMap<K, V>,
    ) -> &'a im_rc::HashMap<K, V> {
        todo!()
    }
}

mod ecs {
    use super::*;
    pub struct Blah<K, V> {
        _p: PhantomData<(K, V)>,
    }
    pub type EcsModel<K, Ty> = fn(Blah<K, Ty>);
}

mod utils {
    use super::*;
    use dioxus_core::prelude::*;

    /// This tiny util wraps your main component with the initializer for the recoil root.
    /// This is useful for small programs and the examples in this crate
    pub fn RecoilApp<T: 'static>(
        root: impl for<'a> Fn(Context<'a, T>) -> VNode,
    ) -> impl for<'a> Fn(Context<'a, T>) -> VNode {
        move |ctx| {
            use_init_recoil_root(ctx, |_| {});
            root(ctx)
        }
    }
}

mod compiletests {}

pub mod error {
    use thiserror::Error as ThisError;
    pub type Result<T, E = Error> = std::result::Result<T, E>;

    #[derive(ThisError, Debug)]
    pub enum Error {
        #[error("Fatal Internal Error: {0}")]
        FatalInternal(&'static str),
        // #[error("Context is missing")]
        // MissingSharedContext,

        // #[error("No event to progress")]
        // NoEvent,

        // #[error("Wrong Properties Type")]
        // WrongProps,

        // #[error("Base scope has not been mounted yet")]
        // NotMounted,
        // #[error("I/O Error: {0}")]
        // BorrowMut(#[from] std::),
        // #[error("eee Error: {0}")]
        // IO(#[from] core::result::),
        #[error("I/O Error: {0}")]
        IO(#[from] std::io::Error),

        #[error(transparent)]
        Other(#[from] anyhow::Error), // source and Display delegate to anyhow::Error
    }
}
