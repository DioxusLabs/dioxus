use std::{any::Any, cell::RefCell, rc::Rc};

/// A wrapper around some generic data that handles the event's state
///
///
/// Prevent this event from continuing to bubble up the tree to parent elements.
///
/// # Example
///
/// ```rust, no_run
/// # use dioxus::prelude::*;
/// rsx! {
///     button {
///         onclick: move |evt: Event<MouseData>| {
///             evt.stop_propagation();
///         }
///     }
/// };
/// ```
pub struct Event<T: 'static + ?Sized> {
    /// The data associated with this event
    pub data: Rc<T>,
    pub(crate) metadata: Rc<RefCell<EventMetadata>>,
}

#[derive(Clone, Copy)]
pub(crate) struct EventMetadata {
    pub(crate) propagates: bool,
    pub(crate) prevent_default: bool,
}

impl<T: ?Sized + 'static> Event<T> {
    /// Create a new event from the inner data
    pub fn new(data: Rc<T>, propagates: bool) -> Self {
        Self {
            data,
            metadata: Rc::new(RefCell::new(EventMetadata {
                propagates,
                prevent_default: false,
            })),
        }
    }
}

impl<T: ?Sized> Event<T> {
    /// Map the event data to a new type
    ///
    /// # Example
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// rsx! {
    ///    button {
    ///       onclick: move |evt: MouseEvent| {
    ///          let data = evt.map(|data| data.client_coordinates());
    ///          println!("{:?}", data.data());
    ///       }
    ///    }
    /// };
    /// ```
    pub fn map<U: 'static, F: FnOnce(&T) -> U>(&self, f: F) -> Event<U> {
        Event {
            data: Rc::new(f(&self.data)),
            metadata: self.metadata.clone(),
        }
    }

    /// Prevent this event from continuing to bubble up the tree to parent elements.
    ///
    /// # Example
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// rsx! {
    ///     button {
    ///         onclick: move |evt: Event<MouseData>| {
    ///             # #[allow(deprecated)]
    ///             evt.cancel_bubble();
    ///         }
    ///     }
    /// };
    /// ```
    #[deprecated = "use stop_propagation instead"]
    pub fn cancel_bubble(&self) {
        self.metadata.borrow_mut().propagates = false;
    }

    /// Check if the event propagates up the tree to parent elements
    pub fn propagates(&self) -> bool {
        self.metadata.borrow().propagates
    }

    /// Prevent this event from continuing to bubble up the tree to parent elements.
    ///
    /// # Example
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// rsx! {
    ///     button {
    ///         onclick: move |evt: Event<MouseData>| {
    ///             evt.stop_propagation();
    ///         }
    ///     }
    /// };
    /// ```
    pub fn stop_propagation(&self) {
        self.metadata.borrow_mut().propagates = false;
    }

    /// Get a reference to the inner data from this event
    ///
    /// ```rust, no_run
    /// # use dioxus::prelude::*;
    /// rsx! {
    ///     button {
    ///         onclick: move |evt: Event<MouseData>| {
    ///             let data = evt.data();
    ///             async move {
    ///                 println!("{:?}", data);
    ///             }
    ///         }
    ///     }
    /// };
    /// ```
    pub fn data(&self) -> Rc<T> {
        self.data.clone()
    }

    /// Prevent the default action of the event.
    ///
    /// # Example
    ///
    /// ```rust
    /// # use dioxus::prelude::*;
    /// fn App() -> Element {
    ///     rsx! {
    ///         a {
    ///             // You can prevent the default action of the event with `prevent_default`
    ///             onclick: move |event| {
    ///                 event.prevent_default();
    ///             },
    ///             href: "https://dioxuslabs.com",
    ///             "don't go to the link"
    ///         }
    ///     }
    /// }
    /// ```
    ///
    /// Note: This must be called synchronously when handling the event. Calling it after the event has been handled will have no effect.
    ///
    /// <div class="warning">
    ///
    /// This method is not available on the LiveView renderer because LiveView handles all events over a websocket which cannot block.
    ///
    /// </div>
    #[track_caller]
    pub fn prevent_default(&self) {
        self.metadata.borrow_mut().prevent_default = true;
    }

    /// Check if the default action of the event is enabled.
    pub fn default_action_enabled(&self) -> bool {
        !self.metadata.borrow().prevent_default
    }
}

impl Event<dyn Any> {
    /// Downcast this `Event<dyn Any>`` to a concrete type
    pub fn downcast<T: 'static>(self) -> Option<Event<T>> {
        let data = self.data.downcast::<T>().ok()?;
        Some(Event {
            metadata: self.metadata.clone(),
            data,
        })
    }
}

impl<T: ?Sized> Clone for Event<T> {
    fn clone(&self) -> Self {
        Self {
            metadata: self.metadata.clone(),
            data: self.data.clone(),
        }
    }
}

impl<T> std::ops::Deref for Event<T> {
    type Target = Rc<T>;
    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for Event<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("UiEvent")
            .field("bubble_state", &self.propagates())
            .field("prevent_default", &!self.default_action_enabled())
            .field("data", &self.data)
            .finish()
    }
}
