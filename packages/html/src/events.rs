use bumpalo::boxed::Box as BumpBox;
use dioxus_core::exports::bumpalo;
use dioxus_core::*;
use std::any::Any;

pub mod on {
    use super::*;
    use std::sync::Arc;
    macro_rules! event_directory {
        ( $(
            $( #[$attr:meta] )*
            $wrapper:ident: [
            // $eventdata:ident($wrapper:ident): [
                $(
                    $( #[$method_attr:meta] )*
                    $name:ident
                )*
            ];
        )* ) => {
            $(
                $(
                    $(#[$method_attr])*
                    pub fn $name<'a, F>(
                        factory: NodeFactory<'a>,
                        mut callback: F,
                    ) -> Listener<'a>
                        where F: FnMut(Arc<$wrapper>) + 'a
                    {
                        let bump = &factory.bump();

                        // we can't allocate unsized in bumpalo's box, so we need to craft the box manually
                        // safety: this is essentially the same as calling Box::new() but manually
                        // The box is attached to the lifetime of the bumpalo allocator
                        let cb: &mut dyn FnMut(Arc<dyn Any + Send + Sync>) = bump.alloc(move |evt: Arc<dyn Any + Send + Sync>| {
                            let event = evt.downcast::<$wrapper>().unwrap();
                            callback(event)
                        });

                        let callback: BumpBox<dyn FnMut(Arc<dyn Any + Send + Sync>) + 'a> = unsafe { BumpBox::from_raw(cb) };

                        // ie oncopy
                        let event_name = stringify!($name);

                        // ie copy
                        let shortname: &'static str = &event_name[2..];

                        let handler = EventHandler {
                            callback: bump.alloc(std::cell::RefCell::new(Some(callback))),
                        };

                        factory.listener(shortname, handler)
                    }
                )*
            )*
        };
    }

    // The Dioxus Synthetic event system
    // todo: move these into the html event system. dioxus accepts *any* event, so having these here doesn't make sense.
    event_directory! {
        ClipboardEvent: [
            /// Called when "copy"
            oncopy

            /// oncut
            oncut

            /// onpaste
            onpaste
        ];

        CompositionEvent: [
            /// oncompositionend
            oncompositionend

            /// oncompositionstart
            oncompositionstart

            /// oncompositionupdate
            oncompositionupdate
        ];

        KeyboardEvent: [
            /// onkeydown
            onkeydown

            /// onkeypress
            onkeypress

            /// onkeyup
            onkeyup
        ];

        FocusEvent: [
            /// onfocus
            onfocus

            /// onblur
            onblur
        ];

        FormEvent: [
            /// onchange
            onchange

            /// oninput handler
            oninput

            /// oninvalid
            oninvalid

            /// onreset
            onreset

            /// onsubmit
            onsubmit
        ];

        /// A synthetic event that wraps a web-style [`MouseEvent`](https://developer.mozilla.org/en-US/docs/Web/API/MouseEvent)
        ///
        ///
        /// The MouseEvent interface represents events that occur due to the user interacting with a pointing device (such as a mouse).
        ///
        /// ## Trait implementation:
        /// ```rust
        ///     fn alt_key(&self) -> bool;
        ///     fn button(&self) -> i16;
        ///     fn buttons(&self) -> u16;
        ///     fn client_x(&self) -> i32;
        ///     fn client_y(&self) -> i32;
        ///     fn ctrl_key(&self) -> bool;
        ///     fn meta_key(&self) -> bool;
        ///     fn page_x(&self) -> i32;
        ///     fn page_y(&self) -> i32;
        ///     fn screen_x(&self) -> i32;
        ///     fn screen_y(&self) -> i32;
        ///     fn shift_key(&self) -> bool;
        ///     fn get_modifier_state(&self, key_code: &str) -> bool;
        /// ```
        ///
        /// ## Event Handlers
        /// - [`onclick`]
        /// - [`oncontextmenu`]
        /// - [`ondoubleclick`]
        /// - [`ondrag`]
        /// - [`ondragend`]
        /// - [`ondragenter`]
        /// - [`ondragexit`]
        /// - [`ondragleave`]
        /// - [`ondragover`]
        /// - [`ondragstart`]
        /// - [`ondrop`]
        /// - [`onmousedown`]
        /// - [`onmouseenter`]
        /// - [`onmouseleave`]
        /// - [`onmousemove`]
        /// - [`onmouseout`]
        /// - [`onmouseover`]
        /// - [`onmouseup`]
        MouseEvent: [
            /// Execute a callback when a button is clicked.
            ///
            /// ## Description
            ///
            /// An element receives a click event when a pointing device button (such as a mouse's primary mouse button)
            /// is both pressed and released while the pointer is located inside the element.
            ///
            /// - Bubbles: Yes
            /// - Cancelable: Yes
            /// - Interface: [`MouseEvent`]
            ///
            /// If the button is pressed on one element and the pointer is moved outside the element before the button
            /// is released, the event is fired on the most specific ancestor element that contained both elements.
            /// `click` fires after both the `mousedown` and `mouseup` events have fired, in that order.
            ///
            /// ## Example
            /// ```
            /// rsx!( button { "click me", onclick: move |_| log::info!("Clicked!`") } )
            /// ```
            ///
            /// ## Reference
            /// - <https://www.w3schools.com/tags/ev_onclick.asp>
            /// - <https://developer.mozilla.org/en-US/docs/Web/API/Element/click_event>
            onclick

            /// oncontextmenu
            oncontextmenu

            /// ondoubleclick
            ondoubleclick

            /// ondrag
            ondrag

            /// ondragend
            ondragend

            /// ondragenter
            ondragenter

            /// ondragexit
            ondragexit

            /// ondragleave
            ondragleave

            /// ondragover
            ondragover

            /// ondragstart
            ondragstart

            /// ondrop
            ondrop

            /// onmousedown
            onmousedown

            /// onmouseenter
            onmouseenter

            /// onmouseleave
            onmouseleave

            /// onmousemove
            onmousemove

            /// onmouseout
            onmouseout

            ///
            onscroll

            /// onmouseover
            ///
            /// Triggered when the users's mouse hovers over an element.
            onmouseover

            /// onmouseup
            onmouseup
        ];

        PointerEvent: [
            /// pointerdown
            onpointerdown

            /// pointermove
            onpointermove

            /// pointerup
            onpointerup

            /// pointercancel
            onpointercancel

            /// gotpointercapture
            ongotpointercapture

            /// lostpointercapture
            onlostpointercapture

            /// pointerenter
            onpointerenter

            /// pointerleave
            onpointerleave

            /// pointerover
            onpointerover

            /// pointerout
            onpointerout
        ];

        SelectionEvent: [
            /// onselect
            onselect
        ];

        TouchEvent: [
            /// ontouchcancel
            ontouchcancel

            /// ontouchend
            ontouchend

            /// ontouchmove
            ontouchmove

            /// ontouchstart
            ontouchstart
        ];

        WheelEvent: [
            ///
            onwheel
        ];

        MediaEvent: [
            ///abort
            onabort

            ///canplay
            oncanplay

            ///canplaythrough
            oncanplaythrough

            ///durationchange
            ondurationchange

            ///emptied
            onemptied

            ///encrypted
            onencrypted

            ///ended
            onended

            ///error
            onerror

            ///loadeddata
            onloadeddata

            ///loadedmetadata
            onloadedmetadata

            ///loadstart
            onloadstart

            ///pause
            onpause

            ///play
            onplay

            ///playing
            onplaying

            ///progress
            onprogress

            ///ratechange
            onratechange

            ///seeked
            onseeked

            ///seeking
            onseeking

            ///stalled
            onstalled

            ///suspend
            onsuspend

            ///timeupdate
            ontimeupdate

            ///volumechange
            onvolumechange

            ///waiting
            onwaiting
        ];

        AnimationEvent: [
            /// onanimationstart
            onanimationstart

            /// onanimationend
            onanimationend

            /// onanimationiteration
            onanimationiteration
        ];

        TransitionEvent: [
            ///
            ontransitionend
        ];

        ToggleEvent: [
            ///
            ontoggle
        ];
    }

    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug)]
    pub struct ClipboardEvent(
        // DOMDataTransfer clipboardData
    );

    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug)]
    pub struct CompositionEvent {
        pub data: String,
    }

    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug)]
    pub struct KeyboardEvent {
        pub char_code: u32,

        /// Identify which "key" was entered.
        ///
        /// This is the best method to use for all languages. They key gets mapped to a String sequence which you can match on.
        /// The key isn't an enum because there are just so many context-dependent keys.
        ///
        /// A full list on which keys to use is available at:
        /// <https://developer.mozilla.org/en-US/docs/Web/API/KeyboardEvent/key/Key_Values>
        ///
        /// # Example
        ///
        /// ```rust
        /// match event.key().as_str() {
        ///     "Esc" | "Escape" => {}
        ///     "ArrowDown" => {}
        ///     "ArrowLeft" => {}
        ///      _ => {}
        /// }
        /// ```
        ///
        pub key: String,

        /// Get the key code as an enum Variant.
        ///
        /// This is intended for things like arrow keys, escape keys, function keys, and other non-international keys.
        /// To match on unicode sequences, use the [`KeyboardEvent::key`] method - this will return a string identifier instead of a limited enum.
        ///
        ///
        /// ## Example
        ///
        /// ```rust
        /// use dioxus::KeyCode;
        /// match event.key_code() {
        ///     KeyCode::Escape => {}
        ///     KeyCode::LeftArrow => {}
        ///     KeyCode::RightArrow => {}
        ///     _ => {}
        /// }
        /// ```
        ///
        pub key_code: KeyCode,

        /// Indicate if the `alt` modifier key was pressed during this keyboard event
        pub alt_key: bool,

        /// Indicate if the `ctrl` modifier key was pressed during this keyboard event
        pub ctrl_key: bool,

        /// Indicate if the `meta` modifier key was pressed during this keyboard event
        pub meta_key: bool,

        /// Indicate if the `shift` modifier key was pressed during this keyboard event
        pub shift_key: bool,

        pub locale: String,

        pub location: usize,

        pub repeat: bool,

        pub which: usize,
        // get_modifier_state: bool,
    }

    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug)]
    pub struct FocusEvent {/* DOMEventInner:  Send + SyncTarget relatedTarget */}

    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug)]
    pub struct FormEvent {
        pub value: String,
        /* DOMEvent:  Send + SyncTarget relatedTarget */
    }

    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug)]
    pub struct MouseEvent {
        pub alt_key: bool,
        pub button: i16,
        pub buttons: u16,
        pub client_x: i32,
        pub client_y: i32,
        pub ctrl_key: bool,
        pub meta_key: bool,
        pub page_x: i32,
        pub page_y: i32,
        pub screen_x: i32,
        pub screen_y: i32,
        pub shift_key: bool,
        // fn get_modifier_state(&self, key_code: &str) -> bool;
    }

    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug)]
    pub struct PointerEvent {
        // Mouse only
        pub alt_key: bool,
        pub button: i16,
        pub buttons: u16,
        pub client_x: i32,
        pub client_y: i32,
        pub ctrl_key: bool,
        pub meta_key: bool,
        pub page_x: i32,
        pub page_y: i32,
        pub screen_x: i32,
        pub screen_y: i32,
        pub shift_key: bool,
        pub pointer_id: i32,
        pub width: i32,
        pub height: i32,
        pub pressure: f32,
        pub tangential_pressure: f32,
        pub tilt_x: i32,
        pub tilt_y: i32,
        pub twist: i32,
        pub pointer_type: String,
        pub is_primary: bool,
        // pub get_modifier_state: bool,
    }

    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug)]
    pub struct SelectionEvent {}

    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug)]
    pub struct TouchEvent {
        pub alt_key: bool,
        pub ctrl_key: bool,
        pub meta_key: bool,
        pub shift_key: bool,
        // get_modifier_state: bool,
        // changedTouches: DOMTouchList,
        // targetTouches: DOMTouchList,
        // touches: DOMTouchList,
    }

    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug)]
    pub struct WheelEvent {
        pub delta_mode: u32,
        pub delta_x: f64,
        pub delta_y: f64,
        pub delta_z: f64,
    }

    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug)]
    pub struct MediaEvent {}

    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug)]
    pub struct ImageEvent {
        pub load_error: bool,
    }

    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug)]
    pub struct AnimationEvent {
        pub animation_name: String,
        pub pseudo_element: String,
        pub elapsed_time: f32,
    }

    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug)]
    pub struct TransitionEvent {
        pub property_name: String,
        pub pseudo_element: String,
        pub elapsed_time: f32,
    }

    #[cfg_attr(feature = "serialize", derive(serde::Serialize, serde::Deserialize))]
    #[derive(Debug)]
    pub struct ToggleEvent {}
}

#[cfg_attr(
    feature = "serialize",
    derive(serde_repr::Serialize_repr, serde_repr::Deserialize_repr)
)]
#[derive(Clone, Copy, Debug)]
#[repr(u8)]
pub enum KeyCode {
    // That key has no keycode, = 0
    // break, = 3
    // backspace / delete, = 8
    // tab, = 9
    // clear, = 12
    // enter, = 13
    // shift, = 16
    // ctrl, = 17
    // alt, = 18
    // pause/break, = 19
    // caps lock, = 20
    // hangul, = 21
    // hanja, = 25
    // escape, = 27
    // conversion, = 28
    // non-conversion, = 29
    // spacebar, = 32
    // page up, = 33
    // page down, = 34
    // end, = 35
    // home, = 36
    // left arrow, = 37
    // up arrow, = 38
    // right arrow, = 39
    // down arrow, = 40
    // select, = 41
    // print, = 42
    // execute, = 43
    // Print Screen, = 44
    // insert, = 45
    // delete, = 46
    // help, = 47
    // 0, = 48
    // 1, = 49
    // 2, = 50
    // 3, = 51
    // 4, = 52
    // 5, = 53
    // 6, = 54
    // 7, = 55
    // 8, = 56
    // 9, = 57
    // :, = 58
    // semicolon (firefox), equals, = 59
    // <, = 60
    // equals (firefox), = 61
    // ß, = 63
    // @ (firefox), = 64
    // a, = 65
    // b, = 66
    // c, = 67
    // d, = 68
    // e, = 69
    // f, = 70
    // g, = 71
    // h, = 72
    // i, = 73
    // j, = 74
    // k, = 75
    // l, = 76
    // m, = 77
    // n, = 78
    // o, = 79
    // p, = 80
    // q, = 81
    // r, = 82
    // s, = 83
    // t, = 84
    // u, = 85
    // v, = 86
    // w, = 87
    // x, = 88
    // y, = 89
    // z, = 90
    // Windows Key / Left ⌘ / Chromebook Search key, = 91
    // right window key, = 92
    // Windows Menu / Right ⌘, = 93
    // sleep, = 95
    // numpad 0, = 96
    // numpad 1, = 97
    // numpad 2, = 98
    // numpad 3, = 99
    // numpad 4, = 100
    // numpad 5, = 101
    // numpad 6, = 102
    // numpad 7, = 103
    // numpad 8, = 104
    // numpad 9, = 105
    // multiply, = 106
    // add, = 107
    // numpad period (firefox), = 108
    // subtract, = 109
    // decimal point, = 110
    // divide, = 111
    // f1, = 112
    // f2, = 113
    // f3, = 114
    // f4, = 115
    // f5, = 116
    // f6, = 117
    // f7, = 118
    // f8, = 119
    // f9, = 120
    // f10, = 121
    // f11, = 122
    // f12, = 123
    // f13, = 124
    // f14, = 125
    // f15, = 126
    // f16, = 127
    // f17, = 128
    // f18, = 129
    // f19, = 130
    // f20, = 131
    // f21, = 132
    // f22, = 133
    // f23, = 134
    // f24, = 135
    // f25, = 136
    // f26, = 137
    // f27, = 138
    // f28, = 139
    // f29, = 140
    // f30, = 141
    // f31, = 142
    // f32, = 143
    // num lock, = 144
    // scroll lock, = 145
    // airplane mode, = 151
    // ^, = 160
    // !, = 161
    // ؛ (arabic semicolon), = 162
    // #, = 163
    // $, = 164
    // ù, = 165
    // page backward, = 166
    // page forward, = 167
    // refresh, = 168
    // closing paren (AZERTY), = 169
    // *, = 170
    // ~ + * key, = 171
    // home key, = 172
    // minus (firefox), mute/unmute, = 173
    // decrease volume level, = 174
    // increase volume level, = 175
    // next, = 176
    // previous, = 177
    // stop, = 178
    // play/pause, = 179
    // e-mail, = 180
    // mute/unmute (firefox), = 181
    // decrease volume level (firefox), = 182
    // increase volume level (firefox), = 183
    // semi-colon / ñ, = 186
    // equal sign, = 187
    // comma, = 188
    // dash, = 189
    // period, = 190
    // forward slash / ç, = 191
    // grave accent / ñ / æ / ö, = 192
    // ?, / or °, = 193
    // numpad period (chrome), = 194
    // open bracket, = 219
    // back slash, = 220
    // close bracket / å, = 221
    // single quote / ø / ä, = 222
    // `, = 223
    // left or right ⌘ key (firefox), = 224
    // altgr, = 225
    // < /git >, left back slash, = 226
    // GNOME Compose Key, = 230
    // ç, = 231
    // XF86Forward, = 233
    // XF86Back, = 234
    // non-conversion, = 235
    // alphanumeric, = 240
    // hiragana/katakana, = 242
    // half-width/full-width, = 243
    // kanji, = 244
    // unlock trackpad (Chrome/Edge), = 251
    // toggle touchpad, = 255
    NA = 0,
    Break = 3,
    Backspace = 8,
    Tab = 9,
    Clear = 12,
    Enter = 13,
    Shift = 16,
    Ctrl = 17,
    Alt = 18,
    Pause = 19,
    CapsLock = 20,
    // hangul, = 21
    // hanja, = 25
    Escape = 27,
    // conversion, = 28
    // non-conversion, = 29
    Space = 32,
    PageUp = 33,
    PageDown = 34,
    End = 35,
    Home = 36,
    LeftArrow = 37,
    UpArrow = 38,
    RightArrow = 39,
    DownArrow = 40,
    // select, = 41
    // print, = 42
    // execute, = 43
    // Print Screen, = 44
    Insert = 45,
    Delete = 46,
    // help, = 47
    Num0 = 48,
    Num1 = 49,
    Num2 = 50,
    Num3 = 51,
    Num4 = 52,
    Num5 = 53,
    Num6 = 54,
    Num7 = 55,
    Num8 = 56,
    Num9 = 57,
    // :, = 58
    // semicolon (firefox), equals, = 59
    // <, = 60
    // equals (firefox), = 61
    // ß, = 63
    // @ (firefox), = 64
    A = 65,
    B = 66,
    C = 67,
    D = 68,
    E = 69,
    F = 70,
    G = 71,
    H = 72,
    I = 73,
    J = 74,
    K = 75,
    L = 76,
    M = 77,
    N = 78,
    O = 79,
    P = 80,
    Q = 81,
    R = 82,
    S = 83,
    T = 84,
    U = 85,
    V = 86,
    W = 87,
    X = 88,
    Y = 89,
    Z = 90,
    LeftWindow = 91,
    RightWindow = 92,
    SelectKey = 93,
    Numpad0 = 96,
    Numpad1 = 97,
    Numpad2 = 98,
    Numpad3 = 99,
    Numpad4 = 100,
    Numpad5 = 101,
    Numpad6 = 102,
    Numpad7 = 103,
    Numpad8 = 104,
    Numpad9 = 105,
    Multiply = 106,
    Add = 107,
    Subtract = 109,
    DecimalPoint = 110,
    Divide = 111,
    F1 = 112,
    F2 = 113,
    F3 = 114,
    F4 = 115,
    F5 = 116,
    F6 = 117,
    F7 = 118,
    F8 = 119,
    F9 = 120,
    F10 = 121,
    F11 = 122,
    F12 = 123,
    // f13, = 124
    // f14, = 125
    // f15, = 126
    // f16, = 127
    // f17, = 128
    // f18, = 129
    // f19, = 130
    // f20, = 131
    // f21, = 132
    // f22, = 133
    // f23, = 134
    // f24, = 135
    // f25, = 136
    // f26, = 137
    // f27, = 138
    // f28, = 139
    // f29, = 140
    // f30, = 141
    // f31, = 142
    // f32, = 143
    NumLock = 144,
    ScrollLock = 145,
    // airplane mode, = 151
    // ^, = 160
    // !, = 161
    // ؛ (arabic semicolon), = 162
    // #, = 163
    // $, = 164
    // ù, = 165
    // page backward, = 166
    // page forward, = 167
    // refresh, = 168
    // closing paren (AZERTY), = 169
    // *, = 170
    // ~ + * key, = 171
    // home key, = 172
    // minus (firefox), mute/unmute, = 173
    // decrease volume level, = 174
    // increase volume level, = 175
    // next, = 176
    // previous, = 177
    // stop, = 178
    // play/pause, = 179
    // e-mail, = 180
    // mute/unmute (firefox), = 181
    // decrease volume level (firefox), = 182
    // increase volume level (firefox), = 183
    Semicolon = 186,
    EqualSign = 187,
    Comma = 188,
    Dash = 189,
    Period = 190,
    ForwardSlash = 191,
    GraveAccent = 192,
    // ?, / or °, = 193
    // numpad period (chrome), = 194
    OpenBracket = 219,
    BackSlash = 220,
    CloseBraket = 221,
    SingleQuote = 222,
    // `, = 223
    // left or right ⌘ key (firefox), = 224
    // altgr, = 225
    // < /git >, left back slash, = 226
    // GNOME Compose Key, = 230
    // ç, = 231
    // XF86Forward, = 233
    // XF86Back, = 234
    // non-conversion, = 235
    // alphanumeric, = 240
    // hiragana/katakana, = 242
    // half-width/full-width, = 243
    // kanji, = 244
    // unlock trackpad (Chrome/Edge), = 251
    // toggle touchpad, = 255
    #[cfg_attr(feature = "serialize", serde(other))]
    Unknown,
}

impl KeyCode {
    pub fn from_raw_code(i: u8) -> Self {
        use KeyCode::*;
        match i {
            8 => Backspace,
            9 => Tab,
            13 => Enter,
            16 => Shift,
            17 => Ctrl,
            18 => Alt,
            19 => Pause,
            20 => CapsLock,
            27 => Escape,
            33 => PageUp,
            34 => PageDown,
            35 => End,
            36 => Home,
            37 => LeftArrow,
            38 => UpArrow,
            39 => RightArrow,
            40 => DownArrow,
            45 => Insert,
            46 => Delete,
            48 => Num0,
            49 => Num1,
            50 => Num2,
            51 => Num3,
            52 => Num4,
            53 => Num5,
            54 => Num6,
            55 => Num7,
            56 => Num8,
            57 => Num9,
            65 => A,
            66 => B,
            67 => C,
            68 => D,
            69 => E,
            70 => F,
            71 => G,
            72 => H,
            73 => I,
            74 => J,
            75 => K,
            76 => L,
            77 => M,
            78 => N,
            79 => O,
            80 => P,
            81 => Q,
            82 => R,
            83 => S,
            84 => T,
            85 => U,
            86 => V,
            87 => W,
            88 => X,
            89 => Y,
            90 => Z,
            91 => LeftWindow,
            92 => RightWindow,
            93 => SelectKey,
            96 => Numpad0,
            97 => Numpad1,
            98 => Numpad2,
            99 => Numpad3,
            100 => Numpad4,
            101 => Numpad5,
            102 => Numpad6,
            103 => Numpad7,
            104 => Numpad8,
            105 => Numpad9,
            106 => Multiply,
            107 => Add,
            109 => Subtract,
            110 => DecimalPoint,
            111 => Divide,
            112 => F1,
            113 => F2,
            114 => F3,
            115 => F4,
            116 => F5,
            117 => F6,
            118 => F7,
            119 => F8,
            120 => F9,
            121 => F10,
            122 => F11,
            123 => F12,
            144 => NumLock,
            145 => ScrollLock,
            186 => Semicolon,
            187 => EqualSign,
            188 => Comma,
            189 => Dash,
            190 => Period,
            191 => ForwardSlash,
            192 => GraveAccent,
            219 => OpenBracket,
            220 => BackSlash,
            221 => CloseBraket,
            222 => SingleQuote,
            _ => Unknown,
        }
    }

    // get the raw code
    pub fn raw_code(&self) -> u32 {
        *self as u32
    }
}

pub(crate) fn _event_meta(event: &UserEvent) -> (bool, EventPriority) {
    use EventPriority::*;

    match event.name {
        // clipboard
        "copy" | "cut" | "paste" => (true, Medium),

        // Composition
        "compositionend" | "compositionstart" | "compositionupdate" => (true, Low),

        // Keyboard
        "keydown" | "keypress" | "keyup" => (true, High),

        // Focus
        "focus" | "blur" => (true, Low),

        // Form
        "change" | "input" | "invalid" | "reset" | "submit" => (true, Medium),

        // Mouse
        "click" | "contextmenu" | "doubleclick" | "drag" | "dragend" | "dragenter" | "dragexit"
        | "dragleave" | "dragover" | "dragstart" | "drop" | "mousedown" | "mouseenter"
        | "mouseleave" | "mouseout" | "mouseover" | "mouseup" => (true, High),

        "mousemove" => (false, Medium),

        // Pointer
        "pointerdown" | "pointermove" | "pointerup" | "pointercancel" | "gotpointercapture"
        | "lostpointercapture" | "pointerenter" | "pointerleave" | "pointerover" | "pointerout" => {
            (true, Medium)
        }

        // Selection
        "select" | "touchcancel" | "touchend" => (true, Medium),

        // Touch
        "touchmove" | "touchstart" => (true, Medium),

        // Wheel
        "scroll" | "wheel" => (false, Medium),

        // Media
        "abort" | "canplay" | "canplaythrough" | "durationchange" | "emptied" | "encrypted"
        | "ended" | "error" | "loadeddata" | "loadedmetadata" | "loadstart" | "pause" | "play"
        | "playing" | "progress" | "ratechange" | "seeked" | "seeking" | "stalled" | "suspend"
        | "timeupdate" | "volumechange" | "waiting" => (true, Medium),

        // Animation
        "animationstart" | "animationend" | "animationiteration" => (true, Medium),

        // Transition
        "transitionend" => (true, Medium),

        // Toggle
        "toggle" => (true, Medium),

        _ => (true, Low),
    }
}
