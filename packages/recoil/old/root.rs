use std::{
    any::Any,
    cell::RefCell,
    collections::HashMap,
    rc::Rc,
    sync::atomic::{AtomicU32, AtomicUsize},
};

use crate::{Atom, AtomBuilder, AtomValue, Readable};

pub type OpaqueConsumerCallback = Box<dyn Any>;

struct Consumer {
    callback: OpaqueConsumerCallback,
}

static SUBSCRIBER_ID: AtomicU32 = AtomicU32::new(0);
fn next_id() -> u32 {
    SUBSCRIBER_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
}

type AtomId = u32;
type ConsumerId = u32;

pub struct RecoilRoot {
    consumers: RefCell<HashMap<AtomId, Box<dyn RecoilSlot>>>,
}

impl RecoilRoot {
    pub(crate) fn new() -> Self {
        Self {
            consumers: Default::default(),
        }
    }

    // This is run by hooks when they register as a listener for a given atom
    // Their ID is stored in the `atom_consumers` map which lets us know which consumers to update
    // When the hook is dropped, they are unregistered from this map
    //
    // If the Atom they're registering as a listener for doesn't exist, then the atom
    // is initialized. Otherwise, the most recent value of the atom is returned
    // pub fn register_readable<T: AtomValue>(&self, atom: &impl Readable<T>) -> Rc<T> {
    //     todo!()
    // }

    /// This registers the updater fn to any updates to the readable
    /// Whenever the readable changes, the updater Fn will be called.
    ///
    /// This also back-propogates changes, meaning components that update an atom
    /// will be updated from their subscription instead of directly within their own hook.
    pub fn subscribe_consumer<T: AtomValue>(
        &self,
        atom: &'static impl Readable<T>,
        updater: impl Fn(),
        // return the value and the consumer's ID
        // the consumer needs to store its ID so it can properly unsubscribe from the value
    ) -> (u32, Rc<T>) {
        let id = next_id();

        // Get the raw static reference of the atom
        let atom_ptr = get_atom_raw_ref(atom);
        let mut consumers = self.consumers.borrow_mut();
        if !consumers.contains_key(&atom_ptr) {
            // Run the atom's initialization
            // let mut b = AtomBuilder::<T>::new();
            // let inital_value = atom.0(&mut b);

            consumers.insert(atom_ptr, HashMap::new());
        }

        todo!()

        // Rc::new(inital_value)
        // todo!()
        // // Get the raw static reference of the atom
        // let atom_ptr = get_atom_raw_ref(atom);

        // // Subcribe this hook_id to this atom
        // let mut consumers = self.consumers.borrow_mut();
        // let hooks = consumers
        //     .get_mut(&atom_ptr)
        //     .expect("Atom must be initialized before being subscribed to");

        // // Coerce into any by wrapping the updater in a box
        // // (yes it's some weird indirection)
        // let any_updater: OpaqueConsumerCallback = Box::new(updater);
        // let consumer = Consumer {
        //     callback: any_updater,
        // };

        // Insert into the map, booting out the old consumer
        // TODO @Jon, make the "Consumer" more efficient, patching its update mechanism in-place
        // hooks.insert(hook_id, consumer);
    }

    pub fn drop_consumer(&self, subscriber_id: u32) {
        // let mut consumers = self.consumers.borrow_mut();
        // let atom_ptr = get_atom_raw_ref(atom);
        // let atoms = consumers.get_mut(&atom_ptr);

        // if let Some(consumers) = atoms {
        //     let entry = consumers.remove_entry(&hook_id);
        //     if let Some(_) = entry {
        //         log::debug!("successfully unsubscribed listener");
        //     } else {
        //         log::debug!("Failure to unsubscribe");
        //     }
        // } else {
        //     log::debug!("Strange error, atoms should be registed if the consumer is being dropped");
        // }
    }

    pub fn update_atom<T: AtomValue>(&self, atom: &impl Readable<T>, new_val: T) {
        // Get the raw static reference of the atom
        // let atom_ptr = get_atom_raw_ref(atom);

        // let mut consumers = self.consumers.borrow_mut();
        // let hooks = consumers
        //     .get_mut(&atom_ptr)
        //     .expect("Atom needs to be registered before trying to update it");

        // let new_val = Rc::new(new_val);
        // for hook in hooks.values_mut() {
        //     let callback: &mut Rc<ConsumerCallback<T>> = hook
        //         .callback
        //         .downcast_mut::<_>()
        //         .expect("Wrong type of atom stored, internal error");

        //     callback(UpdateAction::Regenerate(new_val.clone()));
        // }
    }

    pub fn load_value<T: AtomValue>(&self, atom: &impl Readable<T>) -> Rc<T> {
        todo!()
    }
}

trait RecoilSlot {}

// struct RecoilSlot {
//     consumers: HashMap<u32, Box<dyn Fn()>>,
// }

fn get_atom_raw_ref<T: AtomValue>(atom: &'static impl Readable<T>) -> u32 {
    let atom_ptr = atom as *const _;
    let atom_ptr = atom_ptr as *const u32;
    atom_ptr as u32
}
