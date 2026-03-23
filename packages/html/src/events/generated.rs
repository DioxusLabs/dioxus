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
                    #[doc = "The onmounted event is fired when the element is first added to the DOM. This event gives you a [`MountedData`] object and lets you interact with the raw DOM element."]
                    onmounted => mounted,
                ]]
                Mounted(MountedData),

                #[convert = convert_mouse_data]
                #[events = [
                    #[doc = "Execute a callback when a button is clicked."]
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
                    #[doc = "Called when the mouse wheel is rotated over an element."]
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
