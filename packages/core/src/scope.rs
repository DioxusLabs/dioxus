use crate::context::hooks::Hook;
use crate::inner::*;
use crate::nodes::VNode;
use bumpalo::Bump;
use generational_arena::Index;
use std::{
    any::TypeId, borrow::Borrow, cell::RefCell, future::Future, marker::PhantomData,
    sync::atomic::AtomicUsize,
};

/// Every component in Dioxus is represented by a `Scope`.
///
/// Scopes contain the state for hooks, the component's props, and other lifecycle information.
///
/// Scopes are allocated in a generational arena. As components are mounted/unmounted, they will replace slots of dead components.
/// The actual contents of the hooks, though, will be allocated with the standard allocator. These should not allocate as frequently.
pub struct Scope {
    // These hooks are actually references into the hook arena
    // These two could be combined with "OwningRef" to remove unsafe usage
    // TODO @Jon
    hooks: RefCell<Vec<*mut Hook>>,
    hook_arena: typed_arena::Arena<Hook>,

    // Map to the parent
    parent: Option<Index>,

    props_type: TypeId,
    caller: *const i32,
}

impl Scope {
    // create a new scope from a function
    pub fn new<T: 'static>(f: FC<T>, parent: Option<Index>) -> Self {
        // Capture the props type
        let props_type = TypeId::of::<T>();
        let hook_arena = typed_arena::Arena::new();
        let hooks = RefCell::new(Vec::new());

        let caller = f as *const i32;

        Self {
            hook_arena,
            hooks,
            props_type,
            caller,
            parent,
        }
    }

    pub fn create_context<'runner, T: Properties>(
        &'runner mut self,
        components: &'runner generational_arena::Arena<Scope>,
        props: &'runner T,
    ) -> Context {
        // ) -> Context<T> {
        Context {
            _p: PhantomData {},
            arena: &self.hook_arena,
            hooks: &self.hooks,
            idx: 0.into(),
            // props,
            components,
        }
    }

    /// Create a new context and run the component with references from the Virtual Dom
    /// This function downcasts the function pointer based on the stored props_type
    fn run<T: 'static>(&self, f: FC<T>) {}
}

mod bad_unsafety {
    // todo
    // fn call<'a, T: Properties + 'static>(&'a mut self, val: T) {
    //     if self.props_type == TypeId::of::<T>() {
    //         /*
    //         SAFETY ALERT

    //         This particular usage of transmute is outlined in its docs https://doc.rust-lang.org/std/mem/fn.transmute.html
    //         We hide the generic bound on the function item by casting it to raw pointer. When the function is actually called,
    //         we transmute the function back using the props as reference.

    //         This is safe because we check that the generic type matches before casting.
    //         */
    //         let caller = unsafe { std::mem::transmute::<*const i32, FC<T>>(self.caller) };
    //     // let ctx = self.create_context::<T>();
    //     // // TODO: do something with these nodes
    //     // let nodes = caller(ctx);
    //     } else {
    //         panic!("Do not try to use `call` on Scopes with the wrong props type")
    //     }
    // }
}
