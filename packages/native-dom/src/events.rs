use blitz_dom::{
    BaseDocument, Node, ScrollBehavior as BlitzScrollBehavior,
    ScrollLogicalPosition as BlitzScrollLogicalPosition,
};
use blitz_traits::events::{
    BlitzDragEvent, BlitzKeyEvent, BlitzPointerEvent, BlitzPointerId, BlitzScrollEvent,
    BlitzWheelDelta, BlitzWheelEvent, BlitzWheelPhase, BlitzWheelSource, MouseEventButton,
    PointerKind,
};
use blitz_traits::net::Bytes;
use dioxus_html::{
    AnimationData, BeforeInputData, CancelData, ClipboardData, CompositionData, DataTransfer,
    DragData, FileData, FocusData, FormData, FormValue, HasDataTransferData, HasDragData,
    HasFileData, HasFocusData, HasFormData, HasKeyboardData, HasMouseData, HasPointerData,
    HasScrollData, HasTouchData, HasTouchPointData, HasWheelData, HtmlEventConverter, ImageData,
    KeyboardData, MediaData, MountedData, MountedError, MountedResult, MouseData,
    NativeDataTransfer, NativeFileData, PlatformEventData, PointerData, RenderedElementBacking,
    ResizeData, ScrollBehavior, ScrollData, ScrollLogicalPosition, ScrollToOptions, SelectionData,
    ToggleData, TouchData, TouchPoint, TransitionData, VisibleData, WheelData,
    geometry::{
        ClientPoint, ElementPoint, PagePoint, PixelsRect, PixelsSize, PixelsVector2D, ScreenPoint,
        WheelDelta,
        euclid::{Point2D, Size2D, Vector3D},
    },
    input_data::{MouseButton, MouseButtonSet},
    point_interaction::{
        InteractionElementOffset, InteractionLocation, ModifiersInteraction, PointerInteraction,
    },
};
use dioxus_vdom::CapturedError;
use futures_util::{Stream, stream};
use keyboard_types::{Code, Key, Location, Modifiers};
use std::{
    any::Any,
    cell::{Ref, RefCell, RefMut},
    fmt::Display,
    future::Future,
    path::PathBuf,
    pin::Pin,
    rc::Rc,
};

use crate::NodeId;

pub struct NativeConverter {}

impl HtmlEventConverter for NativeConverter {
    fn convert_cancel_data(&self, _event: &PlatformEventData) -> CancelData {
        unimplemented!("todo: convert_cancel_data in dioxus-native. requires support in blitz")
    }

    fn convert_form_data(&self, event: &PlatformEventData) -> FormData {
        event.downcast::<NativeFormData>().unwrap().clone().into()
    }

    fn convert_mouse_data(&self, event: &PlatformEventData) -> MouseData {
        event
            .downcast::<NativePointerData>()
            .unwrap()
            .clone()
            .into()
    }

    fn convert_keyboard_data(&self, event: &PlatformEventData) -> KeyboardData {
        event
            .downcast::<BlitzKeyboardData>()
            .unwrap()
            .clone()
            .into()
    }

    fn convert_focus_data(&self, _event: &PlatformEventData) -> FocusData {
        NativeFocusData {}.into()
    }

    fn convert_animation_data(&self, _event: &PlatformEventData) -> AnimationData {
        unimplemented!("todo: convert_animation_data in dioxus-native. requires support in blitz")
    }

    fn convert_before_input_data(&self, _event: &PlatformEventData) -> BeforeInputData {
        unimplemented!(
            "todo: convert_before_input_data in dioxus-native. requires support in blitz"
        )
    }

    fn convert_clipboard_data(&self, _event: &PlatformEventData) -> ClipboardData {
        unimplemented!("todo: convert_clipboard_data in dioxus-native. requires support in blitz")
    }

    fn convert_composition_data(&self, _event: &PlatformEventData) -> CompositionData {
        unimplemented!("todo: convert_composition_data in dioxus-native. requires support in blitz")
    }

    fn convert_drag_data(&self, event: &PlatformEventData) -> DragData {
        event.downcast::<NativeDragData>().unwrap().clone().into()
    }

    fn convert_image_data(&self, _event: &PlatformEventData) -> ImageData {
        unimplemented!("todo: convert_image_data in dioxus-native. requires support in blitz")
    }

    fn convert_media_data(&self, _event: &PlatformEventData) -> MediaData {
        unimplemented!("todo: convert_media_data in dioxus-native. requires support in blitz")
    }

    fn convert_mounted_data(&self, event: &PlatformEventData) -> MountedData {
        event.downcast::<NodeHandle>().unwrap().clone().into()
    }

    fn convert_pointer_data(&self, event: &PlatformEventData) -> PointerData {
        event
            .downcast::<NativePointerData>()
            .unwrap()
            .clone()
            .into()
    }

    fn convert_scroll_data(&self, event: &PlatformEventData) -> ScrollData {
        event.downcast::<NativeScrollData>().unwrap().clone().into()
    }

    fn convert_selection_data(&self, _event: &PlatformEventData) -> SelectionData {
        unimplemented!("todo: convert_selection_data in dioxus-native. requires support in blitz")
    }

    fn convert_toggle_data(&self, _event: &PlatformEventData) -> ToggleData {
        unimplemented!("todo: convert_toggle_data in dioxus-native. requires support in blitz")
    }

    fn convert_touch_data(&self, event: &PlatformEventData) -> TouchData {
        event.downcast::<NativeTouchData>().unwrap().clone().into()
    }

    fn convert_transition_data(&self, _event: &PlatformEventData) -> TransitionData {
        unimplemented!("todo: convert_transition_data in dioxus-native. requires support in blitz")
    }

    fn convert_wheel_data(&self, event: &PlatformEventData) -> WheelData {
        event.downcast::<NativeWheelData>().unwrap().clone().into()
    }

    fn convert_resize_data(&self, _event: &PlatformEventData) -> ResizeData {
        unimplemented!("todo: convert_resize_data in dioxus-native. requires support in blitz")
    }

    fn convert_visible_data(&self, _event: &PlatformEventData) -> VisibleData {
        unimplemented!("todo: convert_visible_data in dioxus-native. requires support in blitz")
    }
}

#[derive(Clone)]
pub struct NodeHandle {
    pub(crate) doc: Rc<RefCell<BaseDocument>>,
    pub(crate) node_id: NodeId,
}

impl NodeHandle {
    pub fn node_id(&self) -> NodeId {
        self.node_id
    }

    pub fn doc(&self) -> Ref<'_, BaseDocument> {
        self.doc.borrow()
    }

    /// Returns `None` if the document is currently mutably borrowed.
    /// Use this from background tasks to avoid panicking during event handling.
    pub fn try_doc(&self) -> Option<Ref<'_, BaseDocument>> {
        self.doc.try_borrow().ok()
    }

    pub fn doc_mut(&self) -> RefMut<'_, BaseDocument> {
        self.doc.borrow_mut()
    }

    pub fn node(&self) -> Ref<'_, Node> {
        Ref::map(self.doc.borrow(), |doc| {
            doc.get_node(self.node_id)
                .expect("Node does not exist in the Document")
        })
    }

    pub fn node_mut(&self) -> RefMut<'_, Node> {
        RefMut::map(self.doc.borrow_mut(), |doc| {
            doc.get_node_mut(self.node_id)
                .expect("Node does not exist in the Document")
        })
    }

    fn node_not_exist_err<T>(&self) -> Pin<Box<dyn Future<Output = MountedResult<T>>>> {
        let node_id = self.node_id;
        let err = MountedError::OperationFailed(Box::new(NodeNotExistErr(node_id)));
        Box::pin(async move { Err(err) })
    }

    fn pointer_capture_op(
        &self,
        pointer_id: i32,
        op: impl FnOnce(&mut BaseDocument, BlitzPointerId) -> bool,
    ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        let Some(pointer_id) = BlitzPointerId::from_i32(pointer_id) else {
            let err = MountedError::OperationFailed(Box::new(InvalidPointerIdErr(pointer_id)));
            return Box::pin(async move { Err(err) });
        };
        if !op(&mut self.doc_mut(), pointer_id) {
            return self.node_not_exist_err();
        }
        Box::pin(async { Ok(()) })
    }
}

#[derive(Debug)]
struct InvalidPointerIdErr(i32);
impl Display for InvalidPointerIdErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "No active pointer has the id {}", self.0)
    }
}
impl std::error::Error for InvalidPointerIdErr {}

#[derive(Debug)]
struct NodeNotExistErr(NodeId);
impl Display for NodeNotExistErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The node {} does not exist", self.0)
    }
}
impl std::error::Error for NodeNotExistErr {}

impl RenderedElementBacking for NodeHandle {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn get_scroll_offset(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsVector2D>>>> {
        let Some(scroll_offset) = self
            .doc()
            .get_node(self.node_id)
            .map(|node| *node.scroll_offset())
        else {
            return self.node_not_exist_err();
        };
        Box::pin(async move { Ok(PixelsVector2D::new(scroll_offset.x, scroll_offset.y)) })
    }

    fn get_scroll_size(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsSize>>>> {
        let doc = self.doc();
        let Some(node) = doc.get_node(self.node_id) else {
            drop(doc);
            return self.node_not_exist_err();
        };
        let scroll_width = node.final_layout().scroll_width() as f64;
        let scroll_height = node.final_layout().scroll_height() as f64;
        Box::pin(async move { Ok(PixelsSize::new(scroll_width, scroll_height)) })
    }

    fn get_client_rect(&self) -> Pin<Box<dyn Future<Output = MountedResult<PixelsRect>>>> {
        let Some(bounding_rect) = self.doc_mut().get_client_bounding_rect(self.node_id) else {
            return self.node_not_exist_err();
        };
        let pixels_rect = PixelsRect::new(
            Point2D::new(bounding_rect.x, bounding_rect.y),
            Size2D::new(bounding_rect.width, bounding_rect.height),
        );
        Box::pin(async move { Ok(pixels_rect) })
    }

    /// Scroll the element into view (the backing for both `MountedData::scroll_to` and
    /// `MountedData::scroll_to_with_options`).
    fn scroll_to(
        &self,
        options: ScrollToOptions,
    ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        let node_id = self.node_id;
        let mut doc = self.doc_mut();
        if doc.get_node(node_id).is_none() {
            drop(doc);
            return self.node_not_exist_err();
        }

        let behavior = match options.behavior {
            ScrollBehavior::Smooth => BlitzScrollBehavior::Smooth,
            ScrollBehavior::Instant => BlitzScrollBehavior::Instant,
        };
        let vertical = match options.vertical {
            ScrollLogicalPosition::Start => BlitzScrollLogicalPosition::Start,
            ScrollLogicalPosition::Center => BlitzScrollLogicalPosition::Center,
            ScrollLogicalPosition::End => BlitzScrollLogicalPosition::End,
            ScrollLogicalPosition::Nearest => BlitzScrollLogicalPosition::Nearest,
        };
        let horizontal = match options.horizontal {
            ScrollLogicalPosition::Start => BlitzScrollLogicalPosition::Start,
            ScrollLogicalPosition::Center => BlitzScrollLogicalPosition::Center,
            ScrollLogicalPosition::End => BlitzScrollLogicalPosition::End,
            ScrollLogicalPosition::Nearest => BlitzScrollLogicalPosition::Nearest,
        };
        doc.scroll_into_view(node_id, behavior, vertical, horizontal);

        Box::pin(async { Ok(()) })
    }

    /// Scroll the element's own content to the given offset (the backing for
    /// `MountedData::scroll`). The offset is clamped to the element's scrollable range.
    fn scroll(
        &self,
        coordinates: PixelsVector2D,
        behavior: ScrollBehavior,
    ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        let node_id = self.node_id;
        let mut doc = self.doc_mut();
        if doc.get_node(node_id).is_none() {
            drop(doc);
            return self.node_not_exist_err();
        }

        doc.scroll_to(
            node_id,
            coordinates.x,
            coordinates.y,
            match behavior {
                ScrollBehavior::Smooth => BlitzScrollBehavior::Smooth,
                ScrollBehavior::Instant => BlitzScrollBehavior::Instant,
            },
        );

        Box::pin(async { Ok(()) })
    }

    fn set_focus(&self, focus: bool) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        let mut doc = self.doc_mut();
        if focus {
            // TODO: queue focus events somehow
            doc.set_focus_to(self.node_id);
        } else if doc.get_focussed_node_id() == Some(self.node_id) {
            // Q: Should this only clear focus if the node is focussed?
            // TODO: queue blur events somehow
            doc.clear_focus();
        }

        Box::pin(async { Ok(()) })
    }

    fn set_pointer_capture(
        &self,
        pointer_id: i32,
    ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        let node_id = self.node_id;
        self.pointer_capture_op(pointer_id, |doc, id| doc.set_pointer_capture(node_id, id))
    }

    fn release_pointer_capture(
        &self,
        pointer_id: i32,
    ) -> Pin<Box<dyn Future<Output = MountedResult<()>>>> {
        let node_id = self.node_id;
        self.pointer_capture_op(pointer_id, |doc, id| {
            doc.release_pointer_capture(id, Some(node_id));
            doc.get_node(node_id).is_some()
        })
    }
}

#[derive(Clone, Debug)]
pub struct NativeFormData {
    pub value: String,
    pub values: Vec<(String, FormValue)>,
}

impl HasFormData for NativeFormData {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn value(&self) -> String {
        self.value.clone()
    }

    fn values(&self) -> Vec<(String, FormValue)> {
        self.values.clone()
    }
    fn valid(&self) -> bool {
        // todo: actually implement validation here.
        true
    }
}

impl HasFileData for NativeFormData {
    fn files(&self) -> Vec<dioxus_html::FileData> {
        vec![]
    }
}

#[derive(Clone, Debug)]
pub(crate) struct BlitzKeyboardData(pub(crate) BlitzKeyEvent);

impl ModifiersInteraction for BlitzKeyboardData {
    fn modifiers(&self) -> Modifiers {
        self.0.modifiers
    }
}

impl HasKeyboardData for BlitzKeyboardData {
    fn key(&self) -> Key {
        self.0.key.clone()
    }

    fn code(&self) -> Code {
        self.0.code
    }

    fn location(&self) -> Location {
        self.0.location
    }

    fn is_auto_repeating(&self) -> bool {
        self.0.is_auto_repeating
    }

    fn is_composing(&self) -> bool {
        self.0.is_composing
    }

    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
}

#[derive(Clone)]
pub struct NativePointerData(pub(crate) BlitzPointerEvent);

impl InteractionLocation for NativePointerData {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.0.client_x() as f64, self.0.client_y() as f64)
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(self.0.screen_x() as f64, self.0.screen_y() as f64)
    }

    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(self.0.page_x() as f64, self.0.page_y() as f64)
    }
}

impl InteractionElementOffset for NativePointerData {
    fn element_coordinates(&self) -> ElementPoint {
        ElementPoint::new(self.0.element_x() as f64, self.0.element_y() as f64)
    }
}

impl ModifiersInteraction for NativePointerData {
    fn modifiers(&self) -> Modifiers {
        self.0.mods
    }
}

fn blitz_mouse_button_to_dioxus_mouse_button(button: &MouseEventButton) -> MouseButton {
    match button {
        MouseEventButton::Main => MouseButton::Primary,
        MouseEventButton::Auxiliary => MouseButton::Auxiliary,
        MouseEventButton::Secondary => MouseButton::Secondary,
        MouseEventButton::Fourth => MouseButton::Fourth,
        MouseEventButton::Fifth => MouseButton::Fifth,
    }
}

impl PointerInteraction for NativePointerData {
    fn trigger_button(&self) -> Option<MouseButton> {
        Some(blitz_mouse_button_to_dioxus_mouse_button(&self.0.button))
    }

    fn held_buttons(&self) -> MouseButtonSet {
        dioxus_html::input_data::decode_mouse_button_set(self.0.buttons.bits() as u16)
    }
}
impl HasMouseData for NativePointerData {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
}

impl HasPointerData for NativePointerData {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn is_primary(&self) -> bool {
        self.0.is_primary
    }

    fn pointer_id(&self) -> i32 {
        self.0.id.as_i32()
    }

    fn pointer_type(&self) -> String {
        match self.0.id.kind() {
            PointerKind::Mouse => String::from("mouse"),
            PointerKind::Pen => String::from("pen"),
            PointerKind::Finger => String::from("touch"),
        }
    }

    fn pressure(&self) -> f32 {
        self.0.details.pressure as f32
    }
    fn tangential_pressure(&self) -> f32 {
        self.0.details.tangential_pressure
    }
    fn tilt_x(&self) -> i32 {
        self.0.details.tilt_x as i32
    }
    fn tilt_y(&self) -> i32 {
        self.0.details.tilt_y as i32
    }
    fn twist(&self) -> i32 {
        self.0.details.twist as i32
    }

    // TODO: implement these fields with real values
    fn width(&self) -> f64 {
        1.0
    }
    fn height(&self) -> f64 {
        1.0
    }
}

/// Touch event data exposed to Dioxus Native application code.
///
/// Blitz tracks input via pointer events, so a touch event is generated from the
/// pointer event of the finger that triggered it (`touches_changed`). The full
/// list of concurrent touches is carried on the triggering event's
/// [`BlitzPointerEvent::active_pointers`] list and reported via `touches`.
#[derive(Clone)]
pub struct NativeTouchData(pub(crate) BlitzPointerEvent);

impl NativeTouchData {
    pub fn new(event: BlitzPointerEvent) -> Self {
        Self(event)
    }
}

impl ModifiersInteraction for NativeTouchData {
    fn modifiers(&self) -> Modifiers {
        self.0.mods
    }
}

impl HasTouchData for NativeTouchData {
    fn touches(&self) -> Vec<TouchPoint> {
        // All pointers currently active on the surface (multi-touch).
        self.0
            .active_pointers
            .borrow()
            .iter()
            .map(|event| TouchPoint::new(NativeTouchPointData(event.clone())))
            .collect()
    }

    fn touches_changed(&self) -> Vec<TouchPoint> {
        // Just the touch that triggered this event.
        vec![TouchPoint::new(NativeTouchPointData(self.0.clone()))]
    }

    fn target_touches(&self) -> Vec<TouchPoint> {
        // We don't track a per-touch target, so approximate `targetTouches`
        // (touches that started on the event target) with all active touches.
        self.touches()
    }

    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
}

#[derive(Clone)]
pub struct NativeTouchPointData(BlitzPointerEvent);

impl InteractionLocation for NativeTouchPointData {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.0.client_x() as f64, self.0.client_y() as f64)
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(self.0.screen_x() as f64, self.0.screen_y() as f64)
    }

    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(self.0.page_x() as f64, self.0.page_y() as f64)
    }
}

impl HasTouchPointData for NativeTouchPointData {
    fn identifier(&self) -> i32 {
        match self.0.id.kind() {
            PointerKind::Finger => self.0.id.as_i32(),
            PointerKind::Mouse | PointerKind::Pen => 0,
        }
    }

    fn force(&self) -> f64 {
        self.0.details.pressure
    }

    fn radius(&self) -> ScreenPoint {
        // TODO: expose real touch radius once blitz tracks it
        ScreenPoint::new(1.0, 1.0)
    }

    fn rotation(&self) -> f64 {
        // TODO: expose real touch rotation once blitz tracks it
        0.0
    }

    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
}

#[derive(Clone)]
pub struct NativeFocusData;
impl HasFocusData for NativeFocusData {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
}

#[derive(Clone)]
pub struct NativeScrollData(pub(crate) BlitzScrollEvent);

impl NativeScrollData {
    pub fn new(event: BlitzScrollEvent) -> Self {
        Self(event)
    }
}
impl HasScrollData for NativeScrollData {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn scroll_top(&self) -> f64 {
        self.0.scroll_top
    }

    fn scroll_left(&self) -> f64 {
        self.0.scroll_left
    }

    fn scroll_width(&self) -> i32 {
        self.0.scroll_width
    }

    fn scroll_height(&self) -> i32 {
        self.0.scroll_height
    }

    fn client_width(&self) -> i32 {
        self.0.client_width
    }

    fn client_height(&self) -> i32 {
        self.0.client_height
    }
}

/// A wheel event's data, with what Dioxus's [`WheelData`] has no field for. Reach it
/// from an `onwheel` handler with `let data = event.data();` then
/// `data.downcast::<NativeWheelData>()`.
#[derive(Clone)]
pub struct NativeWheelData(pub(crate) BlitzWheelEvent);

impl NativeWheelData {
    pub fn new(event: BlitzWheelEvent) -> Self {
        Self(event)
    }

    /// Where the event falls in its gesture, as when the fingers land on or lift from
    /// a trackpad. [`BlitzWheelPhase::Moved`] on a device that reports no gestures.
    pub fn phase(&self) -> BlitzWheelPhase {
        self.0.phase
    }

    /// What drives the event, such as a trackpad's momentum after the fingers lift.
    /// [`BlitzWheelSource::Unknown`] where the platform doesn't say.
    pub fn source(&self) -> BlitzWheelSource {
        self.0.source
    }
}

impl HasWheelData for NativeWheelData {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }

    fn delta(&self) -> WheelDelta {
        match self.0.delta {
            BlitzWheelDelta::Lines(x, y) => {
                dioxus_html::geometry::WheelDelta::Lines(Vector3D::new(x, y, 0.0))
            }
            BlitzWheelDelta::Pixels(x, y) => {
                dioxus_html::geometry::WheelDelta::Pixels(Vector3D::new(x, y, 0.0))
            }
        }
    }
}

impl HasMouseData for NativeWheelData {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
}

impl PointerInteraction for NativeWheelData {
    fn trigger_button(&self) -> Option<MouseButton> {
        None
    }

    fn held_buttons(&self) -> MouseButtonSet {
        dioxus_html::input_data::decode_mouse_button_set(self.0.buttons.bits() as u16)
    }
}

impl ModifiersInteraction for NativeWheelData {
    fn modifiers(&self) -> Modifiers {
        self.0.mods
    }
}

impl InteractionElementOffset for NativeWheelData {
    fn element_coordinates(&self) -> ElementPoint {
        ElementPoint::new(self.0.element_x() as f64, self.0.element_y() as f64)
    }
}

impl InteractionLocation for NativeWheelData {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.0.client_x() as f64, self.0.client_y() as f64)
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(self.0.screen_x() as f64, self.0.screen_y() as f64)
    }

    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(self.0.page_x() as f64, self.0.page_y() as f64)
    }
}

pub fn synthetic_click_event(node: &Node, modifiers: Modifiers) -> Box<dyn Any> {
    Box::new(NativePointerData(
        node.synthetic_click_event_data(modifiers),
    ))
}
#[derive(Clone)]
pub struct NativeDragData(pub(crate) BlitzDragEvent);

/// A file being dragged
#[derive(Clone)]
struct BlitzNativeFileData(PathBuf);

impl NativeFileData for BlitzNativeFileData {
    fn name(&self) -> String {
        self.0
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default()
    }

    fn size(&self) -> u64 {
        std::fs::metadata(&self.0).map(|m| m.len()).unwrap_or(0)
    }

    fn last_modified(&self) -> u64 {
        std::fs::metadata(&self.0)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as u64)
            .unwrap_or(0)
    }

    fn content_type(&self) -> Option<String> {
        None
    }

    fn path(&self) -> PathBuf {
        self.0.clone()
    }

    fn inner(&self) -> &dyn Any {
        &self.0
    }

    fn read_bytes(&self) -> Pin<Box<dyn Future<Output = Result<Bytes, CapturedError>> + 'static>> {
        let path = self.0.clone();
        Box::pin(async move {
            std::fs::read(path)
                .map(Bytes::from)
                .map_err(CapturedError::new)
        })
    }

    fn read_string(
        &self,
    ) -> Pin<Box<dyn Future<Output = Result<String, CapturedError>> + 'static>> {
        let path = self.0.clone();
        Box::pin(async move { std::fs::read_to_string(path).map_err(CapturedError::new) })
    }

    fn byte_stream(
        &self,
    ) -> Pin<Box<dyn Stream<Item = Result<Bytes, CapturedError>> + 'static + Send>> {
        use std::io::Read;

        // The state is the open file, or `None` once the stream has ended
        let file = std::fs::File::open(&self.0).map_err(CapturedError::new);
        Box::pin(stream::unfold(Some(file), |file| async move {
            let mut file = match file? {
                Ok(file) => file,
                Err(err) => return Some((Err(err), None)),
            };
            let mut buf = vec![0u8; 64 * 1024];
            match file.read(&mut buf) {
                Ok(0) => None,
                Ok(n) => {
                    buf.truncate(n);
                    Some((Ok(Bytes::from(buf)), Some(Ok(file))))
                }
                Err(err) => Some((Err(CapturedError::new(err)), None)),
            }
        }))
    }
}

impl NativeDataTransfer for NativeDragData {
    fn get_data(&self, format: &str) -> Option<String> {
        self.0.data_transfer().get_data(format)
    }

    fn set_data(&self, format: &str, data: &str) -> Result<(), String> {
        self.0.data_transfer_mut().set_data(format, data)
    }

    fn clear_data(&self, format: Option<&str>) -> Result<(), String> {
        self.0.data_transfer_mut().clear_data(format)
    }

    fn effect_allowed(&self) -> String {
        self.0.data_transfer().effect_allowed_str().to_string()
    }

    fn set_effect_allowed(&self, effect: &str) {
        self.0.data_transfer_mut().set_effect_allowed_str(effect);
    }

    fn drop_effect(&self) -> String {
        self.0.data_transfer().drop_effect().as_str().to_string()
    }

    fn set_drop_effect(&self, effect: &str) {
        self.0.data_transfer_mut().set_drop_effect_str(effect);
    }

    fn files(&self) -> Vec<FileData> {
        self.0
            .data_transfer()
            .files()
            .map(|path| FileData::new(BlitzNativeFileData(path.to_path_buf())))
            .collect()
    }
}

impl HasFileData for NativeDragData {
    fn files(&self) -> Vec<FileData> {
        NativeDataTransfer::files(self)
    }
}

impl HasDataTransferData for NativeDragData {
    fn data_transfer(&self) -> DataTransfer {
        DataTransfer::new(self.clone())
    }
}

impl HasDragData for NativeDragData {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
}

impl ModifiersInteraction for NativeDragData {
    fn modifiers(&self) -> Modifiers {
        self.0.mods
    }
}

impl InteractionLocation for NativeDragData {
    fn client_coordinates(&self) -> ClientPoint {
        ClientPoint::new(self.0.client_x() as f64, self.0.client_y() as f64)
    }

    fn screen_coordinates(&self) -> ScreenPoint {
        ScreenPoint::new(self.0.screen_x() as f64, self.0.screen_y() as f64)
    }

    fn page_coordinates(&self) -> PagePoint {
        PagePoint::new(self.0.page_x() as f64, self.0.page_y() as f64)
    }
}

impl InteractionElementOffset for NativeDragData {
    fn element_coordinates(&self) -> ElementPoint {
        ElementPoint::new(self.0.element_x() as f64, self.0.element_y() as f64)
    }
}

impl PointerInteraction for NativeDragData {
    fn trigger_button(&self) -> Option<MouseButton> {
        Some(blitz_mouse_button_to_dioxus_mouse_button(&self.0.button))
    }

    fn held_buttons(&self) -> MouseButtonSet {
        dioxus_html::input_data::decode_mouse_button_set(self.0.buttons.bits() as u16)
    }
}

impl HasMouseData for NativeDragData {
    fn as_any(&self) -> &dyn Any {
        self as &dyn Any
    }
}

pub fn synthetic_form_event(
    value: impl Into<String>,
    values: impl Into<Vec<(String, FormValue)>>,
) -> Box<dyn Any> {
    Box::new(NativeFormData {
        value: value.into(),
        values: values.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use blitz_traits::events::{
        BlitzPointerId, MouseEventButton, MouseEventButtons, Point, PointerCoords, PointerDetails,
    };

    fn finger_event(id: u64, x: f32, y: f32) -> BlitzPointerEvent {
        BlitzPointerEvent {
            id: BlitzPointerId::finger(id),
            is_primary: id == 0,
            coords: PointerCoords {
                page_x: x,
                page_y: y,
                screen_x: x,
                screen_y: y,
                client_x: x,
                client_y: y,
            },
            button: MouseEventButton::Main,
            buttons: MouseEventButtons::from(MouseEventButton::Main),
            mods: Default::default(),
            details: PointerDetails::default(),
            element: Point::default(),
            active_pointers: Default::default(),
        }
    }

    #[test]
    fn pointer_data_uses_blitz_ids_and_kinds() {
        let mut event = finger_event(7, 1.0, 2.0);
        for (id, kind) in [
            (BlitzPointerId::mouse(), "mouse"),
            (BlitzPointerId::pen(), "pen"),
            (event.id, "touch"),
        ] {
            event.id = id;
            let data = NativePointerData(event.clone());
            assert_eq!(data.pointer_id(), id.as_i32());
            assert_eq!(data.pointer_type(), kind);
        }
    }

    #[test]
    fn touches_reports_all_active_pointers() {
        let f0 = finger_event(0, 10.0, 20.0);
        let f1 = finger_event(1, 30.0, 40.0);

        // The triggering event (second finger down) carries the list of all
        // currently-active pointers.
        let trigger = f1.clone();
        {
            let mut list = trigger.active_pointers.borrow_mut();
            list.push(f0.clone());
            list.push(f1.clone());
        }

        let data = NativeTouchData(trigger);

        // `touches` reports every active pointer ...
        let touches = data.touches();
        assert_eq!(touches.len(), 2);
        let coords: Vec<(f64, f64)> = touches
            .iter()
            .map(|t| {
                let c = t.client_coordinates();
                (c.x, c.y)
            })
            .collect();
        assert!(coords.contains(&(10.0, 20.0)));
        assert!(coords.contains(&(30.0, 40.0)));

        // ... while `changed_touches` reports only the triggering touch.
        assert_eq!(data.touches_changed().len(), 1);
        let changed = data.touches_changed();
        let changed_touch = changed[0].downcast::<NativeTouchPointData>().unwrap();
        assert_eq!(changed_touch.identifier(), f1.id.as_i32());
        let changed_coords = changed[0].client_coordinates();
        assert_eq!((changed_coords.x, changed_coords.y), (30.0, 40.0));
    }

    #[test]
    fn touches_is_empty_when_no_pointers_are_active() {
        // e.g. a `touchend` for the last finger: it has been removed from the
        // active list before dispatch, so `touches` is empty but
        // `changed_touches` still reports the finger that ended.
        let data = NativeTouchData(finger_event(0, 1.0, 2.0));
        assert!(data.touches().is_empty());
        assert_eq!(data.touches_changed().len(), 1);
    }
}
