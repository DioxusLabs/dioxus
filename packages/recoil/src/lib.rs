use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
    hash::Hash,
    marker::PhantomData,
    rc::Rc,
};

pub use api::*;
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

mod traits {
    use dioxus_core::prelude::Context;

    use super::*;
    pub trait FamilyKey: PartialEq + Hash + 'static {}
    impl<T: PartialEq + Hash + 'static> FamilyKey for T {}

    pub trait AtomValue: PartialEq + 'static {}
    impl<T: PartialEq + 'static> AtomValue for T {}

    // Atoms, selectors, and their family variants are readable
    pub trait Readable<T: AtomValue>: Sized + Copy {
        fn use_read<'a>(self, ctx: Context<'a>) -> &'a T {
            hooks::use_read(ctx, self);
            todo!()
        }

        // This returns a future of the value
        // If the atom is currently pending, that future will resolve to pending
        // If the atom is currently ready, the future will immediately resolve
        // if the atom switches from ready to pending, the component will re-run, returning a pending future
        fn use_read_async<'a>(self, ctx: Context<'a>) -> &'a T {
            todo!()
        }

        fn initialize(self, api: &RecoilRoot) -> T {
            todo!()
        }

        // We use the Raw Ptr to the atom
        // TODO: Make sure atoms with the same definitions don't get merged together. I don't think they do, but double check
        fn static_id(self) -> u32;
    }

    // Only atoms and atom families are writable
    // Selectors and selector families are not
    pub trait Writable<T: AtomValue>: Readable<T> + Sized {
        fn use_read_write<'a>(self, ctx: Context<'a>) -> (&'a T, &'a Rc<dyn Fn(T)>) {
            todo!()
        }

        fn use_write<'a>(self, ctx: Context<'a>) -> &'a Rc<dyn Fn(T)> {
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
            todo!()
        }
    }

    impl<T: AtomValue> Writable<T> for &'static Atom<T> {}

    mod compilests {
        use super::*;
        use dioxus_core::prelude::Context;

        const Example: Atom<i32> = |_| 10;

        fn test(ctx: Context) {
            // ensure that atoms are both read and write
            let _ = use_read(ctx, &Example);
            let _ = use_read_write(ctx, &Example);
            let _ = use_write(ctx, &Example);
        }
    }
}

mod atomfamily {
    use super::*;
    pub trait FamilyCollection<K, V> {}
    impl<K, V> FamilyCollection<K, V> for HashMap<K, V> {}

    pub type AtomFamily<K, V, F = HashMap<K, V>> = fn((&K, &V)) -> F;

    pub trait AtomFamilySelector<K: FamilyKey, V: AtomValue> {
        fn select(&'static self, k: &K) -> AtomFamilySelection<K, V> {
            todo!()
        }
    }

    impl<K: FamilyKey, V: AtomValue> AtomFamilySelector<K, V> for AtomFamily<K, V> {
        fn select(&'static self, k: &K) -> AtomFamilySelection<K, V> {
            todo!()
        }
    }

    pub struct AtomFamilySelection<'a, K: FamilyKey, V: AtomValue> {
        root: &'static AtomFamily<K, V>,
        key: &'a K,
    }

    impl<'a, K: FamilyKey, V: AtomValue> Readable<V> for &AtomFamilySelection<'a, K, V> {
        fn static_id(self) -> u32 {
            todo!()
        }
    }

    impl<'a, K: FamilyKey, T: AtomValue> Writable<T> for &AtomFamilySelection<'a, K, T> {}

    mod compiletests {
        use dioxus_core::prelude::Context;

        use super::*;
        const Titles: AtomFamily<u32, &str> = |_| HashMap::new();

        fn test(ctx: Context) {
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
    }

    /// Borrowed selector families are – surprisingly - discouraged.
    /// This is because it's not possible safely memoize these values without keeping old versions around.
    ///
    /// However, it does come in handy to borrow the contents of an item without re-rendering child components.
    pub type SelectorFamilyBorrowed<Key, Value> =
        for<'a> fn(&'a mut SelectorFamilyBuilder, Key) -> &'a Value;

    // impl<'a, K, V: 'a> SelectionSelector<K, V> for fn(&'a mut SelectorFamilyBuilder, K) -> V {}
}

mod api {
    use super::*;

    // pub struct RecoilApi {}
    // impl RecoilApi {
    //     pub fn get<T: AtomValue>(&self, t: &'static Atom<T>) -> Rc<T> {
    //         todo!()
    //     }
    //     pub fn modify<T: PartialEq, O>(
    //         &self,
    //         t: &'static Atom<T>,
    //         f: impl FnOnce(&mut T) -> O,
    //     ) -> O {
    //         todo!()
    //     }
    //     pub fn set<T: AtomValue>(&self, t: &'static Atom<T>, new: T) {
    //         self.modify(t, move |old| *old = new);
    //     }
    // }
}

mod root {
    use std::{
        any::{Any, TypeId},
        collections::{HashSet, VecDeque},
        iter::FromIterator,
    };

    use super::*;
    // use generational_arena::Index as ConsumerId;
    type AtomId = u32;
    type ConsumerId = u32;

    pub struct RecoilContext {
        pub(crate) inner: Rc<RefCell<RecoilRoot>>,
    }

    impl RecoilContext {
        pub fn new() -> Self {
            Self {
                inner: Rc::new(RefCell::new(RecoilRoot::new())),
            }
        }
    }

    // Sometimes memoization means we don't need to re-render components that holds "correct values"
    // IE we consider re-render more expensive than keeping the old value around.
    // We *could* unsafely overwrite this slot, but that's just **asking** for UB (holding a &mut while & is held in components)
    //
    // Instead, we choose to let the hook itself hold onto the Rc<T> by not forcing a render when T is the same.
    // Whenever the component needs to be re-rendered for other reasons, the "get" method will automatically update the Rc<T> to the most recent one.
    pub struct RecoilRoot {
        nodes: HashMap<AtomId, Slot>,
    }

    struct Slot {
        type_id: TypeId,
        source: AtomId,
        value: Rc<dyn Any>,
        consumers: HashMap<ConsumerId, Rc<dyn Fn()>>,
        dependents: HashSet<AtomId>,
    }

    impl RecoilRoot {
        pub(crate) fn new() -> Self {
            Self {
                nodes: Default::default(),
            }
        }

        pub fn subscribe<T: AtomValue>(
            &self,
            readable: impl Readable<T>,
            receiver_fn: Rc<dyn Fn()>,
        ) -> ConsumerId {
            todo!()
        }

        pub fn unsubscribe(&self, id: ConsumerId) {
            todo!()
        }

        /// Directly get the *slot*
        /// All Atoms are held in slots (an Rc)
        ///
        ///
        pub fn try_get_raw<T: AtomValue>(&self, readable: impl Readable<T>) -> Result<Rc<T>> {
            todo!()
        }

        // pub fn try_get<T: AtomValue>(&self, readable: impl Readable<T>) -> Result<&T> {
        //     self.try_get_raw(readable).map(|f| f.as_ref())
        // }

        pub fn try_set<T: AtomValue>(
            &mut self,
            writable: impl Writable<T>,
            new_val: T,
        ) -> crate::error::Result<()> {
            let atom_id = writable.static_id();

            let consumers = match self.nodes.get_mut(&atom_id) {
                Some(slot) => {
                    slot.value = Rc::new(new_val);
                    &slot.consumers
                }
                None => {
                    let value = Slot {
                        type_id: TypeId::of::<T>(),
                        source: atom_id,
                        value: Rc::new(writable.initialize(self)),
                        consumers: Default::default(),
                        dependents: Default::default(),
                    };
                    self.nodes.insert(atom_id, value);
                    &self.nodes.get(&atom_id).unwrap().consumers
                }
            };

            for (_, consumer_fn) in consumers {
                consumer_fn();
            }

            // if it's a an atom or selector, update all the dependents

            Ok(())
        }

        pub fn get<T: AtomValue>(&self, readable: impl Readable<T>) -> Rc<T> {
            todo!()
            // self.try_get(readable).unwrap()
        }

        pub fn set<T: AtomValue>(&mut self, writable: impl Writable<T>, new_val: T) {
            self.try_set(writable, new_val).unwrap();
        }

        /// A slightly dangerous method to manually overwrite any slot given an AtomId
        pub(crate) fn set_by_id<T: AtomValue>(&self, id: AtomId, new_val: T) {}
    }
}

mod hooks {
    use super::*;
    use dioxus_core::prelude::Context;

    pub fn use_init_recoil_root(ctx: Context, cfg: impl Fn(())) {
        ctx.use_create_context(move || RecoilRoot::new())
    }

    /// Gain access to the recoil API directly - set, get, modify, everything
    /// This is the foundational hook in which read/write/modify are built on
    ///
    /// This does not subscribe the component to *any* updates
    ///
    /// You can use this method to create controllers that perform much more complex actions than set/get
    /// However, be aware that "getting" values through this hook will not subscribe the component to any updates.
    pub fn use_recoil_api<'a, F: 'a>(
        ctx: Context<'a>,
        f: impl Fn(Rc<RecoilRoot>) -> F + 'static,
    ) -> &F {
        let g = ctx.use_context::<RecoilContext>();
        let api = g.inner.clone();
        todo!()
    }

    pub fn use_write<'a, T: AtomValue>(
        ctx: Context<'a>,
        writable: impl Writable<T>,
    ) -> &'a Rc<dyn Fn(T)> {
        let api = use_recoil_api(ctx, |f| f);
        ctx.use_hook(
            move || {
                let api = api.clone();
                let raw_id = writable.static_id();
                Rc::new(move |new_val| api.set_by_id(raw_id, new_val)) as Rc<dyn Fn(T)>
            },
            move |hook| &*hook,
            |hook| {},
        )
    }

    /// Read the atom and get the Rc directly to the Atom's slot
    /// This is useful if you need the memoized Atom value. However, Rc<T> is not as easy to
    /// work with as
    pub fn use_read_raw<'a, T: AtomValue>(ctx: Context<'a>, readable: impl Readable<T>) -> &Rc<T> {
        struct ReadHook<T> {
            value: Rc<T>,
            consumer_id: u32,
        }

        let api = use_recoil_api(ctx, |api| api);
        ctx.use_hook(
            move || {
                let update = ctx.schedule_update();
                let val = api.try_get_raw(readable).unwrap();
                let id = api.subscribe(readable, Rc::new(update));
                ReadHook {
                    value: val,
                    consumer_id: id,
                }
            },
            move |hook| {
                let val = api.try_get_raw(readable).unwrap();
                hook.value = val;
                &hook.value
            },
            |hook| {
                api.unsubscribe(hook.consumer_id);
            },
        )
    }

    ///
    pub fn use_read<'a, T: AtomValue>(ctx: Context<'a>, readable: impl Readable<T>) -> &'a T {
        use_read_raw(ctx, readable).as_ref()
    }

    /// Use an atom in both read and write modes - only available for atoms and family selections (not selectors)
    /// This is equivalent to calling both `use_read` and `use_write`, but saves you the hassle and repitition
    ///
    /// ```
    /// const Title: Atom<&str> = |_| "hello";
    /// //...
    /// let (title, set_title) = use_read_write(ctx, &Title);
    ///
    /// // equivalent to:
    /// let (title, set_title) = (use_read(ctx, &Title), use_write(ctx, &Title));
    /// ```
    pub fn use_read_write<'a, T: AtomValue + 'static>(
        ctx: Context<'a>,
        writable: impl Writable<T>,
    ) -> (&'a T, &'a Rc<dyn Fn(T)>) {
        (use_read(ctx, writable), use_write(ctx, writable))
    }

    /// Use a family collection directly
    /// !! Any changes to the family will cause this subscriber to update
    /// Try not to put this at the very top-level of your app.
    pub fn use_read_family<'a, K, V, C: FamilyCollection<K, V>>(
        ctx: Context<'a>,
        t: &AtomFamily<K, V, C>,
    ) -> &'a C {
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
        root: impl for<'a> Fn(Context<'a>, &'a T) -> DomTree,
    ) -> impl for<'a> Fn(Context<'a>, &'a T) -> DomTree {
        move |ctx, props| {
            use_init_recoil_root(ctx, |_| {});
            root(ctx, props)
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
