use dioxus_core::{AttributeValue, ElementId, Template, WriteMutations};

/// A [`WriteMutations`] wrapper that intercepts `mounted` event listeners and queues them on
/// the Rust side instead of forwarding them to the inner writer. After the edits have been
/// applied to the DOM, the renderer drains the queue with [`Self::take_mounted_events`] and
/// dispatches the mounted events against the now-live elements.
///
/// Both the web and desktop renderers wrap their mutation writers in this type so mounted
/// events behave identically on the two platforms.
pub struct QueueMountedEvents<W> {
    inner: W,
    mounted_event_listeners: Vec<ElementId>,
}

impl<W> QueueMountedEvents<W> {
    /// Wrap a mutation writer, intercepting its `mounted` event listeners.
    pub fn new(inner: W) -> Self {
        Self {
            inner,
            mounted_event_listeners: Vec::new(),
        }
    }

    /// Take the mounted event listeners queued since the last call.
    pub fn take_mounted_events(&mut self) -> Vec<ElementId> {
        std::mem::take(&mut self.mounted_event_listeners)
    }
}

impl<W> std::ops::Deref for QueueMountedEvents<W> {
    type Target = W;

    fn deref(&self) -> &W {
        &self.inner
    }
}

impl<W> std::ops::DerefMut for QueueMountedEvents<W> {
    fn deref_mut(&mut self) -> &mut W {
        &mut self.inner
    }
}

impl<W: WriteMutations> WriteMutations for QueueMountedEvents<W> {
    fn append_children(&mut self, id: ElementId, m: usize) {
        self.inner.append_children(id, m);
    }

    fn assign_node_id(&mut self, path: &'static [u8], id: ElementId) {
        self.inner.assign_node_id(path, id);
    }

    fn create_placeholder(&mut self, id: ElementId) {
        self.inner.create_placeholder(id);
    }

    fn create_text_node(&mut self, value: &str, id: ElementId) {
        self.inner.create_text_node(value, id);
    }

    fn load_template(&mut self, template: Template, index: usize, id: ElementId) {
        self.inner.load_template(template, index, id);
    }

    fn replace_node_with(&mut self, id: ElementId, m: usize) {
        self.inner.replace_node_with(id, m);
    }

    fn replace_placeholder_with_nodes(&mut self, path: &'static [u8], m: usize) {
        self.inner.replace_placeholder_with_nodes(path, m);
    }

    fn insert_nodes_after(&mut self, id: ElementId, m: usize) {
        self.inner.insert_nodes_after(id, m);
    }

    fn insert_nodes_before(&mut self, id: ElementId, m: usize) {
        self.inner.insert_nodes_before(id, m);
    }

    fn set_attribute(
        &mut self,
        name: &'static str,
        ns: Option<&'static str>,
        value: &AttributeValue,
        id: ElementId,
    ) {
        self.inner.set_attribute(name, ns, value, id);
    }

    fn set_node_text(&mut self, value: &str, id: ElementId) {
        self.inner.set_node_text(value, id);
    }

    fn create_event_listener(&mut self, name: &'static str, id: ElementId) {
        if name == "mounted" {
            self.mounted_event_listeners.push(id);
            return;
        }

        self.inner.create_event_listener(name, id);
    }

    fn remove_event_listener(&mut self, name: &'static str, id: ElementId) {
        // Mounted listeners were never forwarded to the inner writer, so there is nothing to remove.
        if name == "mounted" {
            return;
        }

        self.inner.remove_event_listener(name, id);
    }

    fn remove_node(&mut self, id: ElementId) {
        self.inner.remove_node(id);
    }

    fn push_root(&mut self, id: ElementId) {
        self.inner.push_root(id);
    }
}
