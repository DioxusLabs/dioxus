use super::*;

#[doc(hidden)]
#[macro_export]
macro_rules! with_html_event_groups {
    ($macro:ident) => {
        $macro! {
            enum Event {
                #[convert = convert_animation_data]
                #[events = [
                    onanimationstart => animationstart,
                    onanimationend => animationend,
                    onanimationiteration => animationiteration,
                ]]
                Animation(AnimationData),

                #[convert = convert_cancel_data]
                #[events = [
                    oncancel => cancel,
                ]]
                Cancel(CancelData),

                #[convert = convert_clipboard_data]
                #[events = [
                    oncopy => copy,
                    oncut => cut,
                    onpaste => paste,
                ]]
                Clipboard(ClipboardData),

                #[convert = convert_composition_data]
                #[events = [
                    oncompositionstart => compositionstart,
                    oncompositionend => compositionend,
                    oncompositionupdate => compositionupdate,
                ]]
                Composition(CompositionData),

                #[convert = convert_drag_data]
                #[events = [
                    ondrag => drag,
                    ondragend => dragend,
                    ondragenter => dragenter,
                    ondragexit => dragexit,
                    ondragleave => dragleave,
                    ondragover => dragover,
                    ondragstart => dragstart,
                    ondrop => drop,
                ]]
                Drag(DragData),

                #[convert = convert_focus_data]
                #[events = [
                    onfocus => focus,
                    onfocusout => focusout,
                    onfocusin => focusin,
                    onblur => blur,
                ]]
                Focus(FocusData),

                #[convert = convert_form_data]
                #[events = [
                    onchange => change,
                    /// The `oninput` event is fired when the value of a `<input>`, `<select>`, or `<textarea>` element is changed.
                    ///
                    /// There are two main approaches to updating your input element:
                    /// 1) Controlled inputs directly update the value of the input element as the user interacts with the element
                    ///
                    /// ```rust
                    /// use dioxus::prelude::*;
                    ///
                    /// fn App() -> Element {
                    ///     let mut value = use_signal(|| "hello world".to_string());
                    ///
                    ///     rsx! {
                    ///         input {
                    ///             value: "{value}",
                    ///             oninput: move |event| value.set(event.value())
                    ///         }
                    ///         button {
                    ///             onclick: move |_| value.write().clear(),
                    ///             "Clear"
                    ///         }
                    ///     }
                    /// }
                    /// ```
                    ///
                    /// 2) Uncontrolled inputs just read the value of the input element as it changes
                    ///
                    /// ```rust
                    /// use dioxus::prelude::*;
                    ///
                    /// fn App() -> Element {
                    ///     rsx! {
                    ///         input {
                    ///             oninput: move |event| println!("{}", event.value()),
                    ///         }
                    ///     }
                    /// }
                    /// ```
                    oninput => input,
                    oninvalid => invalid,
                    onreset => reset,
                    onsubmit => submit,
                ]]
                Form(FormData),

                #[convert = convert_image_data]
                #[events = [
                    onerror => error,
                    onload => load,
                ]]
                Image(ImageData),

                #[convert = convert_keyboard_data]
                #[events = [
                    onkeydown => keydown,
                    onkeypress => keypress,
                    onkeyup => keyup,
                ]]
                Keyboard(KeyboardData),

                #[convert = convert_media_data]
                #[events = [
                    onabort => abort,
                    oncanplay => canplay,
                    oncanplaythrough => canplaythrough,
                    ondurationchange => durationchange,
                    onemptied => emptied,
                    onencrypted => encrypted,
                    onended => ended,
                    onloadeddata => loadeddata,
                    onloadedmetadata => loadedmetadata,
                    onloadstart => loadstart,
                    onpause => pause,
                    onplay => play,
                    onplaying => playing,
                    onprogress => progress,
                    onratechange => ratechange,
                    onseeked => seeked,
                    onseeking => seeking,
                    onstalled => stalled,
                    onsuspend => suspend,
                    ontimeupdate => timeupdate,
                    onvolumechange => volumechange,
                    onwaiting => waiting,
                ]]
                #[raw = [interruptbegin, interruptend, loadend, timeout]]
                Media(MediaData),

                #[convert = convert_mounted_data]
                #[events = [
                    #[doc(alias = "ref")]
                    #[doc(alias = "createRef")]
                    #[doc(alias = "useRef")]
                    /// The onmounted event is fired when the element is first added to the DOM. This event gives you a [`MountedData`] object and lets you interact with the raw DOM element.
                    ///
                    /// This event is fired once per element. If you need to access the element multiple times, you can store the [`MountedData`] object in a [`use_signal`](https://docs.rs/dioxus-hooks/latest/dioxus_hooks/fn.use_signal.html) hook and use it as needed.
                    ///
                    /// # Examples
                    ///
                    /// ```rust, no_run
                    /// # use dioxus::prelude::*;
                    /// fn App() -> Element {
                    ///     let mut header_element = use_signal(|| None);
                    ///
                    ///     rsx! {
                    ///         div {
                    ///             h1 {
                    ///                 // The onmounted event will run the first time the h1 element is mounted
                    ///                 onmounted: move |element| header_element.set(Some(element.data())),
                    ///                 "Scroll to top example"
                    ///             }
                    ///
                    ///             for i in 0..100 {
                    ///                 div { "Item {i}" }
                    ///             }
                    ///
                    ///             button {
                    ///                 // When you click the button, if the header element has been mounted, we scroll to that element
                    ///                 onclick: move |_| async move {
                    ///                     if let Some(header) = header_element.cloned() {
                    ///                         let _ = header.scroll_to(ScrollBehavior::Smooth).await;
                    ///                     }
                    ///                 },
                    ///                 "Scroll to top"
                    ///             }
                    ///         }
                    ///     }
                    /// }
                    /// ```
                    ///
                    /// The `MountedData` struct contains cross platform APIs that work on the desktop, mobile, liveview and web platforms. For the web platform, you can also downcast the `MountedData` event to the `web-sys::Element` type for more web specific APIs:
                    ///
                    /// ```rust, ignore
                    /// use dioxus::prelude::*;
                    /// use dioxus_web::WebEventExt; // provides [`as_web_event()`] method
                    ///
                    /// fn App() -> Element {
                    ///     rsx! {
                    ///         div {
                    ///             id: "some-id",
                    ///             onmounted: move |element| {
                    ///                 // You can use the web_event trait to downcast the element to a web specific event. For the mounted event, this will be a web_sys::Element
                    ///                 let web_sys_element = element.as_web_event();
                    ///                 assert_eq!(web_sys_element.id(), "some-id");
                    ///             }
                    ///         }
                    ///     }
                    /// }
                    /// ```
                    onmounted => mounted,
                ]]
                Mounted(MountedData),

                #[convert = convert_mouse_data]
                #[events = [
                    /// Execute a callback when a button is clicked.
                    ///
                    /// ## Description
                    ///
                    /// An element receives a click event when a pointing device button (such as a mouse's primary mouse button)
                    /// is both pressed and released while the pointer is located inside the element.
                    ///
                    /// - Bubbles: Yes
                    /// - Cancelable: Yes
                    /// - Interface(InteData): [`MouseEvent`]
                    ///
                    /// If the button is pressed on one element and the pointer is moved outside the element before the button
                    /// is released, the event is fired on the most specific ancestor element that contained both elements.
                    /// `click` fires after both the `mousedown` and `mouseup` events have fired, in that order.
                    ///
                    /// ## Example
                    /// ```rust, ignore
                    /// rsx!( button { onclick: move |_| tracing::info!("Clicked!"), "click me" } )
                    /// ```
                    ///
                    /// ## Reference
                    /// - <https://www.w3schools.com/tags/ev_onclick.asp>
                    /// - <https://developer.mozilla.org/en-US/docs/Web/API/Element/click_event>
                    onclick => click,
                    oncontextmenu => contextmenu,
                    #[deprecated(since = "0.5.0", note = "use ondoubleclick instead")]
                    ondblclick => dblclick,
                    #[doc(alias = "ondblclick")]
                    ondoubleclick => dblclick,
                    onmousedown => mousedown,
                    onmouseenter => mouseenter,
                    onmouseleave => mouseleave,
                    onmousemove => mousemove,
                    onmouseout => mouseout,
                    /// Triggered when the users's mouse hovers over an element.
                    onmouseover => mouseover,
                    onmouseup => mouseup,
                ]]
                #[raw = [doubleclick]]
                Mouse(MouseData),

                #[convert = convert_pointer_data]
                #[events = [
                    onpointerdown => pointerdown,
                    onpointermove => pointermove,
                    onpointerup => pointerup,
                    onpointercancel => pointercancel,
                    ongotpointercapture => gotpointercapture,
                    onlostpointercapture => lostpointercapture,
                    onpointerenter => pointerenter,
                    onpointerleave => pointerleave,
                    onpointerover => pointerover,
                    onpointerout => pointerout,
                    onauxclick => auxclick,
                ]]
                #[raw = [pointerlockchange, pointerlockerror]]
                Pointer(PointerData),

                #[convert = convert_resize_data]
                #[events = [
                    onresize => resize,
                ]]
                Resize(ResizeData),

                #[convert = convert_scroll_data]
                #[events = [
                    onscroll => scroll,
                    onscrollend => scrollend,
                ]]
                Scroll(ScrollData),

                #[convert = convert_selection_data]
                #[events = [
                    onselect => select,
                    onselectstart => selectstart,
                    onselectionchange => selectionchange,
                ]]
                Selection(SelectionData),

                #[convert = convert_toggle_data]
                #[events = [
                    ontoggle => toggle,
                    onbeforetoggle => beforetoggle,
                ]]
                Toggle(ToggleData),

                #[convert = convert_touch_data]
                #[events = [
                    ontouchstart => touchstart,
                    ontouchmove => touchmove,
                    ontouchend => touchend,
                    ontouchcancel => touchcancel,
                ]]
                Touch(TouchData),

                #[convert = convert_transition_data]
                #[events = [
                    ontransitionend => transitionend,
                ]]
                Transition(TransitionData),

                #[convert = convert_visible_data]
                #[events = [
                    onvisible => visible,
                ]]
                Visible(VisibleData),

                #[convert = convert_wheel_data]
                #[events = [
                    /// Called when the mouse wheel is rotated over an element.
                    onwheel => wheel,
                ]]
                Wheel(WheelData),
            }
        }
    };
}

macro_rules! expand_html_event_converter {
    (
        enum Event {
            $(
                #[convert = $converter:ident]
                #[events = [
                    $(
                        $( #[$attr:meta] )*
                        $name:ident => $raw:ident,
                    )*
                ]]
                $(#[raw = [$($raw_only:ident),* $(,)?]])?
                $group:ident($data:ident),
            )*
        }
    ) => {
        /// A converter between a platform specific event and a general event. All code in a renderer that has a large binary size should be placed in this trait. Each of these functions should be snipped in high levels of optimization.
        pub trait HtmlEventConverter: Send + Sync {
            $(
                fn $converter(&self, event: &PlatformEventData) -> $data;
            )*
        }

        $(
            impl From<&PlatformEventData> for $data {
                fn from(val: &PlatformEventData) -> Self {
                    with_event_converter(|c| c.$converter(val))
                }
            }
        )*
    };
}

#[cfg(feature = "serialize")]
macro_rules! expand_html_event_deserialize {
    (
        enum Event {
            $(
                #[convert = $converter:ident]
                #[events = [
                    $(
                        $( #[$attr:meta] )*
                        $name:ident => $raw:ident,
                    )*
                ]]
                $(#[raw = [$($raw_only:ident),* $(,)?]])?
                $group:ident($data:ident),
            )*
        }
    ) => {
        pub(crate) fn deserialize_raw_event(
            name: &str,
            data: &serde_json::Value,
        ) -> Result<Option<crate::transit::EventData>, serde_json::Error> {
            #[inline]
            fn de<'de, F>(f: &'de serde_json::Value) -> Result<F, serde_json::Error>
            where
                F: serde::Deserialize<'de>,
            {
                F::deserialize(f)
            }

            Ok(match name {
                $(
                    $( stringify!($raw) )|* $($(| stringify!($raw_only))*)? => {
                        Some(expand_html_event_deserialize!(@deserialize $group, data))
                    }
                )*
                _ => None,
            })
        }
    };
    (@deserialize Mounted, $data:ident) => {
        crate::transit::EventData::Mounted
    };
    (@deserialize $group:ident, $data:ident) => {
        crate::transit::EventData::$group(de($data)?)
    };
}

macro_rules! expand_html_event_listeners {
    (
        enum Event {
            $(
                #[convert = $converter:ident]
                #[events = [
                    $(
                        $( #[$attr:meta] )*
                        $name:ident => $raw:ident,
                    )*
                ]]
                $(#[raw = [$($raw_only:ident),* $(,)?]])?
                $group:ident($data:ident),
            )*
        }
    ) => {
        $(
            impl_event! {
                $data;
                $(
                    #[doc = concat!(stringify!($name))]
                    $( #[$attr] )*
                    $name: concat!("on", stringify!($raw));
                )*
            }
        )*
    };
}

with_html_event_groups!(expand_html_event_converter);
#[cfg(feature = "serialize")]
with_html_event_groups!(expand_html_event_deserialize);
with_html_event_groups!(expand_html_event_listeners);
