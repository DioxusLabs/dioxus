#![allow(non_upper_case_globals)]

use dioxus_core::prelude::IntoAttributeValue;
use dioxus_core::HasAttributes;
use dioxus_html_internal_macro::impl_extension_attributes;

use crate::AttributeDescription;

#[cfg(feature = "hot-reload-context")]
macro_rules! mod_method_mapping {
    (
        $matching:ident;
        $(#[$attr:meta])*
        $name:ident;
    ) => {
        if $matching == stringify!($name) {
            return Some((stringify!($name), None));
        }
    };
    (
        $matching:ident;
        $(#[$attr:meta])*
        $name:ident: $lit:literal;
    ) => {
        if $matching == stringify!($name) {
            return Some(($lit, None));
        }
    };
    (
        $matching:ident;
        $(#[$attr:meta])*
        $name:ident: $lit:literal in $ns:literal;
    ) => {
        if $matching == stringify!($name) {
            return Some(($lit, Some($ns)));
        }
    };
    (
        $matching:ident;
        $(#[$attr:meta])*
        $name:ident in $ns:literal;
    ) => {
        if $matching == stringify!($name) {
            return Some((stringify!($name), Some($ns)));
        }
    };
}

#[cfg(feature = "html-to-rsx")]
macro_rules! html_to_rsx_attribute_mapping {
    (
        $matching:ident;
        $(#[$attr:meta])*
        $name:ident;
    ) => {
        if $matching == stringify!($name) {
            return Some(stringify!($name));
        }
    };
    (
        $matching:ident;
        $(#[$attr:meta])*
        $name:ident: $lit:literal;
    ) => {
        if $matching == stringify!($lit) {
            return Some(stringify!($name));
        }
    };
    (
        $matching:ident;
        $(#[$attr:meta])*
        $name:ident: $lit:literal in $ns:literal;
    ) => {
        if $matching == stringify!($lit) {
            return Some(stringify!($name));
        }
    };
    (
        $matching:ident;
        $(#[$attr:meta])*
        $name:ident in $ns:literal;
    ) => {
        if $matching == stringify!($name) {
            return Some(stringify!($name));
        }
    };
}

macro_rules! mod_methods {
    (
        @base
        $(#[$mod_attr:meta])*
        $mod:ident;
        $fn:ident;
        $fn_html_to_rsx:ident;
        $(
            $(#[$attr:meta])*
            $name:ident $(: $(no-$alias:ident)? $js_name:literal)? $(in $ns:literal)?;
        )+
    ) => {
        $(#[$mod_attr])*
        pub mod $mod {
            use super::*;
            $(
                mod_methods! {
                    @attr
                    $(#[$attr])*
                    $name $(: $(no-$alias)? $js_name)? $(in $ns)?;
                }
            )+
        }

        #[cfg(feature = "hot-reload-context")]
        pub(crate) fn $fn(attr: &str) -> Option<(&'static str, Option<&'static str>)> {
            $(
                mod_method_mapping! {
                    attr;
                    $name $(: $js_name)? $(in $ns)?;
                }
            )*
            None
        }

        #[cfg(feature = "html-to-rsx")]
        #[doc = "Converts an HTML attribute to an RSX attribute"]
        pub(crate) fn $fn_html_to_rsx(html: &str) -> Option<&'static str> {
            $(
                html_to_rsx_attribute_mapping! {
                    html;
                    $name $(: $js_name)? $(in $ns)?;
                }
            )*
            None
        }

        impl_extension_attributes![$mod { $($name,)* }];
    };

    (
        @attr
        $(#[$attr:meta])*
        $name:ident $(: no-alias $js_name:literal)? $(in $ns:literal)?;
    ) => {
        $(#[$attr])*
        ///
        /// ## Usage in rsx
        ///
        /// ```rust, ignore
        /// # use dioxus::prelude::*;
        #[doc = concat!("let ", stringify!($name), " = \"value\";")]
        ///
        /// rsx! {
        ///     // Attributes need to be under the element they modify
        ///     div {
        ///         // Attributes are followed by a colon and then the value of the attribute
        #[doc = concat!("        ", stringify!($name), ": \"value\"")]
        ///     }
        ///     div {
        ///         // Or you can use the shorthand syntax if you have a variable in scope that has the same name as the attribute
        #[doc = concat!("        ", stringify!($name), ",")]
        ///     }
        /// };
        /// ```
        pub const $name: AttributeDescription = mod_methods! { $name $(: $js_name)? $(in $ns)?; };
    };

    (
        @attr
        $(#[$attr:meta])*
        $name:ident $(: $js_name:literal)? $(in $ns:literal)?;
    ) => {
        $(#[$attr])*
        ///
        /// ## Usage in rsx
        ///
        /// ```rust, ignore
        /// # use dioxus::prelude::*;
        #[doc = concat!("let ", stringify!($name), " = \"value\";")]
        ///
        /// rsx! {
        ///     // Attributes need to be under the element they modify
        ///     div {
        ///         // Attributes are followed by a colon and then the value of the attribute
        #[doc = concat!("        ", stringify!($name), ": \"value\"")]
        ///     }
        ///     div {
        ///         // Or you can use the shorthand syntax if you have a variable in scope that has the same name as the attribute
        #[doc = concat!("        ", stringify!($name), ",")]
        ///     }
        /// };
        /// ```
        $(
            #[doc(alias = $js_name)]
        )?
        pub const $name: AttributeDescription = mod_methods! { $name $(: $js_name)? $(in $ns)?; };
    };

    // Rename the incoming ident and apply a custom namespace
    ( $name:ident: $lit:literal in $ns:literal; ) => { ($lit, Some($ns), false) };

    // Custom namespace
    ( $name:ident in $ns:literal; ) => { (stringify!($name), Some($ns), false) };

    // Rename the incoming ident
    ( $name:ident: $lit:literal; ) => { ($lit, None, false ) };

    // Don't rename the incoming ident
    ( $name:ident; ) => { (stringify!($name), None, false) };
}

mod_methods! {
    @base

    global_attributes;
    map_global_attributes;
    map_html_global_attributes_to_rsx;

    /// Prevent the default action for this element.
    ///
    /// For more information, see the MDN docs:
    /// <https://developer.mozilla.org/en-US/docs/Web/API/Event/preventDefault>
    prevent_default: "dioxus-prevent-default";


    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/accesskey>
    accesskey;


    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/autocapitalize>
    autocapitalize;


    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/autofocus>
    autofocus;

    /// The HTML class attribute is used to specify a class for an HTML element.
    ///
    /// ## Details
    /// Multiple HTML elements can share the same class.
    ///
    /// The class global attribute is a space-separated list of the case-sensitive classes of the element.
    /// Classes allow CSS and Javascript to select and access specific elements via the class selectors or
    /// functions like the DOM method document.getElementsByClassName.
    ///
    /// ## Multiple Classes
    ///
    /// If you include multiple classes in a single element dioxus will automatically join them with a space.
    ///
    /// ```rust
    /// # use dioxus::prelude::*;
    /// rsx! {
    ///     div {
    ///         class: "my-class",
    ///         class: "my-other-class"
    ///     }
    /// };
    /// ```
    ///
    /// ## Optional Classes
    ///
    /// You can include optional attributes with an unterminated if statement as the value of the attribute. This is very useful for conditionally applying css classes:
    ///
    /// ```rust
    /// # use dioxus::prelude::*;
    /// rsx! {
    ///     div {
    ///         class: if true {
    ///             "my-class"
    ///         },
    ///         class: if false {
    ///             "my-other-class"
    ///         }
    ///     }
    /// };
    /// ```
    ///
    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/class>
    class;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/contenteditable>
    contenteditable;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/data>
    data;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/dir>
    dir;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/draggable>
    draggable;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/enterkeyhint>
    enterkeyhint;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/exportparts>
    exportparts;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/hidden>
    hidden;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/id>
    id;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/inputmode>
    inputmode;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/is>
    is;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/itemid>
    itemid;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/itemprop>
    itemprop;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/itemref>
    itemref;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/itemscope>
    itemscope;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/itemtype>
    itemtype;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/lang>
    lang;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/nonce>
    nonce;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/part>
    part;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/popover>
    popover;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/role>
    role;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/slot>
    slot;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/spellcheck>
    spellcheck;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/style>
    style;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/tabindex>
    tabindex;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/title>
    title;

    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/translate>
    translate;


    /// dangerous_inner_html is Dioxus's replacement for using innerHTML in the browser DOM. In general, setting
    /// HTML from code is risky because it’s easy to inadvertently expose your users to a cross-site scripting (XSS)
    /// attack. So, you can set HTML directly from Dioxus, but you have to type out dangerous_inner_html to remind
    /// yourself that it’s dangerous
    dangerous_inner_html;

    // This macro creates an explicit method call for each of the style attributes.
    //
    // The left token specifies the name of the attribute in the rsx! macro, and the right string literal specifies the
    // actual name of the attribute generated.
    //
    // This roughly follows the html spec

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/align-content>
    align_content: "align-content" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/align-items>
    align_items: "align-items" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/align-self>
    align_self: "align-self" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/alignment-adjust>
    alignment_adjust: "alignment-adjust" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/alignment-baseline>
    alignment_baseline: "alignment-baseline" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/all>
    all in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/alt>
    alt in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation>
    animation in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-delay>
    animation_delay: "animation-delay" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-direction>
    animation_direction: "animation-direction" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-duration>
    animation_duration: "animation-duration" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-fill-mode>
    animation_fill_mode: "animation-fill-mode" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-iteration-count>
    animation_iteration_count: "animation-iteration-count" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-name>
    animation_name: "animation-name" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-play-state>
    animation_play_state: "animation-play-state" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-timing-function>
    animation_timing_function: "animation-timing-function" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/azimuth>
    azimuth in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/backdrop-filter>
    backdrop_filter: "backdrop-filter" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/backface-visibility>
    backface_visibility: "backface-visibility" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background>
    background in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-attachment>
    background_attachment: "background-attachment" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-clip>
    background_clip: "background-clip" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-color>
    background_color: "background-color" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-image>
    background_image: "background-image" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-origin>
    background_origin: "background-origin" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-position>
    background_position: "background-position" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-repeat>
    background_repeat: "background-repeat" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-size>
    background_size: "background-size" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-blend-mode>
    background_blend_mode: "background-blend-mode" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/baseline-shift>
    baseline_shift: "baseline-shift" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/bleed>
    bleed in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/bookmark-label>
    bookmark_label: "bookmark-label" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/bookmark-level>
    bookmark_level: "bookmark-level" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/bookmark-state>
    bookmark_state: "bookmark-state" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border>
    border in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-color>
    border_color: "border-color" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-style>
    border_style: "border-style" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-width>
    border_width: "border-width" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom>
    border_bottom: "border-bottom" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom-color>
    border_bottom_color: "border-bottom-color" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom-style>
    border_bottom_style: "border-bottom-style" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom-width>
    border_bottom_width: "border-bottom-width" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-left>
    border_left: "border-left" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-left-color>
    border_left_color: "border-left-color" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-left-style>
    border_left_style: "border-left-style" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-left-width>
    border_left_width: "border-left-width" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-right>
    border_right: "border-right" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-right-color>
    border_right_color: "border-right-color" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-right-style>
    border_right_style: "border-right-style" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-right-width>
    border_right_width: "border-right-width" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top>
    border_top: "border-top" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top-color>
    border_top_color: "border-top-color" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top-style>
    border_top_style: "border-top-style" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top-width>
    border_top_width: "border-top-width" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-collapse>
    border_collapse: "border-collapse" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image>
    border_image: "border-image" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image-outset>
    border_image_outset: "border-image-outset" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image-repeat>
    border_image_repeat: "border-image-repeat" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image-slice>
    border_image_slice: "border-image-slice" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image-source>
    border_image_source: "border-image-source" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image-width>
    border_image_width: "border-image-width" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-radius>
    border_radius: "border-radius" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom-left-radius>
    border_bottom_left_radius: "border-bottom-left-radius" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom-right-radius>
    border_bottom_right_radius: "border-bottom-right-radius" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top-left-radius>
    border_top_left_radius: "border-top-left-radius" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top-right-radius>
    border_top_right_radius: "border-top-right-radius" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-spacing>
    border_spacing: "border-spacing" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/bottom>
    bottom in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/box-decoration-break>
    box_decoration_break: "box-decoration-break" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/box-shadow>
    box_shadow: "box-shadow" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/box-sizing>
    box_sizing: "box-sizing" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/box-snap>
    box_snap: "box-snap" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/break-after>
    break_after: "break-after" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/break-before>
    break_before: "break-before" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/break-inside>
    break_inside: "break-inside" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/buffered-rendering>
    buffered_rendering: "buffered-rendering" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/caption-side>
    caption_side: "caption-side" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/clear>
    clear in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/clear-side>
    clear_side: "clear-side" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/clip>
    clip in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/clip-path>
    clip_path: "clip-path" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/clip-rule>
    clip_rule: "clip-rule" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color>
    color in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-adjust>
    color_adjust: "color-adjust" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-correction>
    color_correction: "color-correction" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-interpolation>
    color_interpolation: "color-interpolation" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-interpolation-filters>
    color_interpolation_filters: "color-interpolation-filters" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-profile>
    color_profile: "color-profile" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-rendering>
    color_rendering: "color-rendering" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-fill>
    column_fill: "column-fill" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-gap>
    column_gap: "column-gap" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-rule>
    column_rule: "column-rule" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-rule-color>
    column_rule_color: "column-rule-color" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-rule-style>
    column_rule_style: "column-rule-style" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-rule-width>
    column_rule_width: "column-rule-width" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-span>
    column_span: "column-span" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/columns>
    columns in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-count>
    column_count: "column-count" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-width>
    column_width: "column-width" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/contain>
    contain in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/content>
    content in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/counter-increment>
    counter_increment: "counter-increment" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/counter-reset>
    counter_reset: "counter-reset" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/counter-set>
    counter_set: "counter-set" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/cue>
    cue in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/cue-after>
    cue_after: "cue-after" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/cue-before>
    cue_before: "cue-before" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/cursor>
    cursor in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/direction>
    direction in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/display>
    display in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/display-inside>
    display_inside: "display-inside" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/display-outside>
    display_outside: "display-outside" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/display-extras>
    display_extras: "display-extras" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/display-box>
    display_box: "display-box" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/dominant-baseline>
    dominant_baseline: "dominant-baseline" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/elevation>
    elevation in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/empty-cells>
    empty_cells: "empty-cells" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/enable-background>
    enable_background: "enable-background" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/fill>
    fill in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/fill-opacity>
    fill_opacity: "fill-opacity" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/fill-rule>
    fill_rule: "fill-rule" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/filter>
    filter in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/float>
    float in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/float-defer-column>
    float_defer_column: "float-defer-column" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/float-defer-page>
    float_defer_page: "float-defer-page" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/float-offset>
    float_offset: "float-offset" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/float-wrap>
    float_wrap: "float-wrap" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flow-into>
    flow_into: "flow-into" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flow-from>
    flow_from: "flow-from" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex>
    flex in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-basis>
    flex_basis: "flex-basis" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-grow>
    flex_grow: "flex-grow" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-shrink>
    flex_shrink: "flex-shrink" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-flow>
    flex_flow: "flex-flow" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-direction>
    flex_direction: "flex-direction" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-wrap>
    flex_wrap: "flex-wrap" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flood-color>
    flood_color: "flood-color" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flood-opacity>
    flood_opacity: "flood-opacity" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font>
    font in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-family>
    font_family: "font-family" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-size>
    font_size: "font-size" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-stretch>
    font_stretch: "font-stretch" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-style>
    font_style: "font-style" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-weight>
    font_weight: "font-weight" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-feature-settings>
    font_feature_settings: "font-feature-settings" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-kerning>
    font_kerning: "font-kerning" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-language-override>
    font_language_override: "font-language-override" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-size-adjust>
    font_size_adjust: "font-size-adjust" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-synthesis>
    font_synthesis: "font-synthesis" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant>
    font_variant: "font-variant" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-alternates>
    font_variant_alternates: "font-variant-alternates" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-caps>
    font_variant_caps: "font-variant-caps" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-east-asian>
    font_variant_east_asian: "font-variant-east-asian" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-ligatures>
    font_variant_ligatures: "font-variant-ligatures" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-numeric>
    font_variant_numeric: "font-variant-numeric" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-position>
    font_variant_position: "font-variant-position" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/footnote-policy>
    footnote_policy: "footnote-policy" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/glyph-orientation-horizontal>
    glyph_orientation_horizontal: "glyph-orientation-horizontal" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/glyph-orientation-vertical>
    glyph_orientation_vertical: "glyph-orientation-vertical" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid>
    grid in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-auto-flow>
    grid_auto_flow: "grid-auto-flow" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-auto-columns>
    grid_auto_columns: "grid-auto-columns" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-auto-rows>
    grid_auto_rows: "grid-auto-rows" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-template>
    grid_template: "grid-template" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-template-areas>
    grid_template_areas: "grid-template-areas" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-template-columns>
    grid_template_columns: "grid-template-columns" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-template-rows>
    grid_template_rows: "grid-template-rows" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-area>
    grid_area: "grid-area" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-column>
    grid_column: "grid-column" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-column-start>
    grid_column_start: "grid-column-start" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-column-end>
    grid_column_end: "grid-column-end" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-row>
    grid_row: "grid-row" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-row-start>
    grid_row_start: "grid-row-start" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-row-end>
    grid_row_end: "grid-row-end" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hanging-punctuation>
    hanging_punctuation: "hanging-punctuation" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/height>
    height in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphenate-character>
    hyphenate_character: "hyphenate-character" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphenate-limit-chars>
    hyphenate_limit_chars: "hyphenate-limit-chars" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphenate-limit-last>
    hyphenate_limit_last: "hyphenate-limit-last" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphenate-limit-lines>
    hyphenate_limit_lines: "hyphenate-limit-lines" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphenate-limit-zone>
    hyphenate_limit_zone: "hyphenate-limit-zone" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphens>
    hyphens in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/icon>
    icon in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/image-orientation>
    image_orientation: "image-orientation" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/image-resolution>
    image_resolution: "image-resolution" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/image-rendering>
    image_rendering: "image-rendering" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ime>
    ime in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ime-align>
    ime_align: "ime-align" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ime-mode>
    ime_mode: "ime-mode" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ime-offset>
    ime_offset: "ime-offset" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ime-width>
    ime_width: "ime-width" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/initial-letters>
    initial_letters: "initial-letters" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/inline-box-align>
    inline_box_align: "inline-box-align" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/isolation>
    isolation in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/justify-content>
    justify_content: "justify-content" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/justify-items>
    justify_items: "justify-items" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/justify-self>
    justify_self: "justify-self" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/kerning>
    kerning in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/left>
    left in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/letter-spacing>
    letter_spacing: "letter-spacing" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/lighting-color>
    lighting_color: "lighting-color" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-box-contain>
    line_box_contain: "line-box-contain" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-break>
    line_break: "line-break" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-grid>
    line_grid: "line-grid" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-height>
    line_height: "line-height" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-slack>
    line_slack: "line-slack" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-snap>
    line_snap: "line-snap" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/list-style>
    list_style: "list-style" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/list-style-image>
    list_style_image: "list-style-image" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/list-style-position>
    list_style_position: "list-style-position" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/list-style-type>
    list_style_type: "list-style-type" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/margin>
    margin in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/margin-bottom>
    margin_bottom: "margin-bottom" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/margin-left>
    margin_left: "margin-left" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/margin-right>
    margin_right: "margin-right" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/margin-top>
    margin_top: "margin-top" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker>
    marker in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-end>
    marker_end: "marker-end" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-mid>
    marker_mid: "marker-mid" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-pattern>
    marker_pattern: "marker-pattern" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-segment>
    marker_segment: "marker-segment" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-start>
    marker_start: "marker-start" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-knockout-left>
    marker_knockout_left: "marker-knockout-left" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-knockout-right>
    marker_knockout_right: "marker-knockout-right" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-side>
    marker_side: "marker-side" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marks>
    marks in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marquee-direction>
    marquee_direction: "marquee-direction" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marquee-play-count>
    marquee_play_count: "marquee-play-count" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marquee-speed>
    marquee_speed: "marquee-speed" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marquee-style>
    marquee_style: "marquee-style" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask>
    mask in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-image>
    mask_image: "mask-image" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-repeat>
    mask_repeat: "mask-repeat" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-position>
    mask_position: "mask-position" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-clip>
    mask_clip: "mask-clip" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-origin>
    mask_origin: "mask-origin" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-size>
    mask_size: "mask-size" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box>
    mask_box: "mask-box" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box-outset>
    mask_box_outset: "mask-box-outset" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box-repeat>
    mask_box_repeat: "mask-box-repeat" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box-slice>
    mask_box_slice: "mask-box-slice" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box-source>
    mask_box_source: "mask-box-source" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box-width>
    mask_box_width: "mask-box-width" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-type>
    mask_type: "mask-type" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/max-height>
    max_height: "max-height" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/max-lines>
    max_lines: "max-lines" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/max-width>
    max_width: "max-width" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/min-height>
    min_height: "min-height" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/min-width>
    min_width: "min-width" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mix-blend-mode>
    mix_blend_mode: "mix-blend-mode" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/nav-down>
    nav_down: "nav-down" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/nav-index>
    nav_index: "nav-index" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/nav-left>
    nav_left: "nav-left" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/nav-right>
    nav_right: "nav-right" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/nav-up>
    nav_up: "nav-up" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/object-fit>
    object_fit: "object-fit" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/object-position>
    object_position: "object-position" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/offset-after>
    offset_after: "offset-after" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/offset-before>
    offset_before: "offset-before" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/offset-end>
    offset_end: "offset-end" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/offset-start>
    offset_start: "offset-start" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/opacity>
    opacity in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/order>
    order in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/orphans>
    orphans in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/outline>
    outline in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/outline-color>
    outline_color: "outline-color" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/outline-style>
    outline_style: "outline-style" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/outline-width>
    outline_width: "outline-width" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/outline-offset>
    outline_offset: "outline-offset" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/overflow>
    overflow in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/overflow-x>
    overflow_x: "overflow-x" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/overflow-y>
    overflow_y: "overflow-y" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/overflow-style>
    overflow_style: "overflow-style" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/overflow-wrap>
    overflow_wrap: "overflow-wrap" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/padding>
    padding in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/padding-bottom>
    padding_bottom: "padding-bottom" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/padding-left>
    padding_left: "padding-left" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/padding-right>
    padding_right: "padding-right" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/padding-top>
    padding_top: "padding-top" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/page>
    page in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/page-break-after>
    page_break_after: "page-break-after" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/page-break-before>
    page_break_before: "page-break-before" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/page-break-inside>
    page_break_inside: "page-break-inside" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/paint-order>
    paint_order: "paint-order" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pause>
    pause in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pause-after>
    pause_after: "pause-after" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pause-before>
    pause_before: "pause-before" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/perspective>
    perspective in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/perspective-origin>
    perspective_origin: "perspective-origin" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pitch>
    pitch in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pitch-range>
    pitch_range: "pitch-range" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/play-during>
    play_during: "play-during" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pointer-events>
    pointer_events: "pointer-events" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/position>
    position in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/quotes>
    quotes in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/region-fragment>
    region_fragment: "region-fragment" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/resize>
    resize in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/rest>
    rest in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/rest-after>
    rest_after: "rest-after" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/rest-before>
    rest_before: "rest-before" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/richness>
    richness in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/right>
    right in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ruby-align>
    ruby_align: "ruby-align" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ruby-merge>
    ruby_merge: "ruby-merge" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ruby-position>
    ruby_position: "ruby-position" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-behavior>
    scroll_behavior: "scroll-behavior" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-snap-coordinate>
    scroll_snap_coordinate: "scroll-snap-coordinate" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-snap-destination>
    scroll_snap_destination: "scroll-snap-destination" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-snap-points-x>
    scroll_snap_points_x: "scroll-snap-points-x" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-snap-points-y>
    scroll_snap_points_y: "scroll-snap-points-y" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-snap-type>
    scroll_snap_type: "scroll-snap-type" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-image-threshold>
    shape_image_threshold: "shape-image-threshold" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-inside>
    shape_inside: "shape-inside" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-margin>
    shape_margin: "shape-margin" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-outside>
    shape_outside: "shape-outside" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-padding>
    shape_padding: "shape-padding" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-rendering>
    shape_rendering: "shape-rendering" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/size>
    size in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speak>
    speak in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speak-as>
    speak_as: "speak-as" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speak-header>
    speak_header: "speak-header" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speak-numeral>
    speak_numeral: "speak-numeral" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speak-punctuation>
    speak_punctuation: "speak-punctuation" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speech-rate>
    speech_rate: "speech-rate" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stop-color>
    stop_color: "stop-color" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stop-opacity>
    stop_opacity: "stop-opacity" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stress>
    stress in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/string-set>
    string_set: "string-set" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke>
    stroke in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-dasharray>
    stroke_dasharray: "stroke-dasharray" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-dashoffset>
    stroke_dashoffset: "stroke-dashoffset" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-linecap>
    stroke_linecap: "stroke-linecap" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-linejoin>
    stroke_linejoin: "stroke-linejoin" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-miterlimit>
    stroke_miterlimit: "stroke-miterlimit" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-opacity>
    stroke_opacity: "stroke-opacity" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-width>
    stroke_width: "stroke-width" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/tab-size>
    tab_size: "tab-size" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/table-layout>
    table_layout: "table-layout" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-align>
    text_align: "text-align" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-align-all>
    text_align_all: "text-align-all" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-align-last>
    text_align_last: "text-align-last" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-anchor>
    text_anchor: "text-anchor" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-combine-upright>
    text_combine_upright: "text-combine-upright" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration>
    text_decoration: "text-decoration" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration-color>
    text_decoration_color: "text-decoration-color" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration-line>
    text_decoration_line: "text-decoration-line" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration-style>
    text_decoration_style: "text-decoration-style" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration-skip>
    text_decoration_skip: "text-decoration-skip" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-emphasis>
    text_emphasis: "text-emphasis" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-emphasis-color>
    text_emphasis_color: "text-emphasis-color" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-emphasis-style>
    text_emphasis_style: "text-emphasis-style" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-emphasis-position>
    text_emphasis_position: "text-emphasis-position" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-emphasis-skip>
    text_emphasis_skip: "text-emphasis-skip" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-height>
    text_height: "text-height" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-indent>
    text_indent: "text-indent" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-justify>
    text_justify: "text-justify" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-orientation>
    text_orientation: "text-orientation" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-overflow>
    text_overflow: "text-overflow" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-rendering>
    text_rendering: "text-rendering" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-shadow>
    text_shadow: "text-shadow" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-size-adjust>
    text_size_adjust: "text-size-adjust" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-space-collapse>
    text_space_collapse: "text-space-collapse" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-spacing>
    text_spacing: "text-spacing" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-transform>
    text_transform: "text-transform" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-underline-position>
    text_underline_position: "text-underline-position" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-wrap>
    text_wrap: "text-wrap" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/top>
    top in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/touch-action>
    touch_action: "touch-action" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transform>
    transform in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transform-box>
    transform_box: "transform-box" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transform-origin>
    transform_origin: "transform-origin" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transform-style>
    transform_style: "transform-style" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transition>
    transition in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transition-delay>
    transition_delay: "transition-delay" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transition-duration>
    transition_duration: "transition-duration" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transition-property>
    transition_property: "transition-property" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/unicode-bidi>
    unicode_bidi: "unicode-bidi" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/vector-effect>
    vector_effect: "vector-effect" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/vertical-align>
    vertical_align: "vertical-align" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/visibility>
    visibility in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-balance>
    voice_balance: "voice-balance" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-duration>
    voice_duration: "voice-duration" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-family>
    voice_family: "voice-family" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-pitch>
    voice_pitch: "voice-pitch" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-range>
    voice_range: "voice-range" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-rate>
    voice_rate: "voice-rate" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-stress>
    voice_stress: "voice-stress" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-volume>
    voice_volume: "voice-volume" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/volume>
    volume in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/white-space>
    white_space: "white-space" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/widows>
    widows in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/width>
    width in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/will-change>
    will_change: "will-change" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/word-break>
    word_break: "word-break" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/word-spacing>
    word_spacing: "word-spacing" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/word-wrap>
    word_wrap: "word-wrap" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/wrap-flow>
    wrap_flow: "wrap-flow" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/wrap-through>
    wrap_through: "wrap-through" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/writing-mode>
    writing_mode: "writing-mode" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/gap>
    gap in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/list-style-type>
    list_styler_type: "list-style-type" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/row-gap>
    row_gap: "row-gap" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transition-timing-function>
    transition_timing_function: "transition-timing-function" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/user-select>
    user_select: "user-select" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/-webkit-user-select>
    webkit_user_select: "-webkit-user-select" in "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/z-index>
    z_index: "z-index" in "style";

    // area attribute

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-current>
    aria_current: "aria-current";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-details>
    aria_details: "aria-details";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-disabled>
    aria_disabled: "aria-disabled";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-hidden>
    aria_hidden: "aria-hidden";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-invalid>
    aria_invalid: "aria-invalid";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-keyshortcuts>
    aria_keyshortcuts: "aria-keyshortcuts";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-label>
    aria_label: "aria-label";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-roledescription>
    aria_roledescription: "aria-roledescription";

// Widget Attributes

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-autocomplete>
    aria_autocomplete: "aria-autocomplete";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-checked>
    aria_checked: "aria-checked";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-expanded>
    aria_expanded: "aria-expanded";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-haspopup>
    aria_haspopup: "aria-haspopup";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-level>
    aria_level: "aria-level";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-modal>
    aria_modal: "aria-modal";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-multiline>
    aria_multiline: "aria-multiline";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-multiselectable>
    aria_multiselectable: "aria-multiselectable";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-orientation>
    aria_orientation: "aria-orientation";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-placeholder>
    aria_placeholder: "aria-placeholder";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-pressed>
    aria_pressed: "aria-pressed";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-readonly>
    aria_readonly: "aria-readonly";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-required>
    aria_required: "aria-required";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-selected>
    aria_selected: "aria-selected";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-sort>
    aria_sort: "aria-sort";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-valuemax>
    aria_valuemax: "aria-valuemax";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-valuemin>
    aria_valuemin: "aria-valuemin";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-valuenow>
    aria_valuenow: "aria-valuenow";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-valuetext>
    aria_valuetext: "aria-valuetext";

// Live Region Attributes

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-atomic>
    aria_atomic: "aria-atomic";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-busy>
    aria_busy: "aria-busy";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-live>
    aria_live: "aria-live";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-relevant>
    aria_relevant: "aria-relevant";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-dropeffect>
    aria_dropeffect: "aria-dropeffect";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-grabbed>
    aria_grabbed: "aria-grabbed";

// Relationship Attributes

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-activedescendant>
    aria_activedescendant: "aria-activedescendant";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-colcount>
    aria_colcount: "aria-colcount";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-colindex>
    aria_colindex: "aria-colindex";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-colspan>
    aria_colspan: "aria-colspan";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-controls>
    aria_controls: "aria-controls";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-describedby>
    aria_describedby: "aria-describedby";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-errormessage>
    aria_errormessage: "aria-errormessage";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-flowto>
    aria_flowto: "aria-flowto";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-labelledby>
    aria_labelledby: "aria-labelledby";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-owns>
    aria_owns: "aria-owns";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-posinset>
    aria_posinset: "aria-posinset";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-rowcount>
    aria_rowcount: "aria-rowcount";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-rowindex>
    aria_rowindex: "aria-rowindex";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-rowspan>
    aria_rowspan: "aria-rowspan";

    /// <https://developer.mozilla.org/en-US/docs/Web/Accessibility/ARIA/Attributes/aria-setsize>
    aria_setsize: "aria-setsize";
}

mod_methods! {
    @base
    svg_attributes;
    map_svg_attributes;
    map_html_svg_attributes_to_rsx;

    /// Prevent the default action for this element.
    ///
    /// For more information, see the MDN docs:
    /// <https://developer.mozilla.org/en-US/docs/Web/API/Event/preventDefault>
    prevent_default: "dioxus-prevent-default";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/accent-height>
    accent_height: "accent-height";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/accumulate>
    accumulate;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/additive>
    additive;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/alignment-baseline>
    alignment_baseline: "alignment-baseline";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/alphabetic>
    alphabetic;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/amplitude>
    amplitude;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/arabic-form>
    arabic_form: "arabic-form";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/ascent>
    ascent;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/attributeName>
    attribute_name: "attributeName";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/attributeType>
    attribute_type: "attributeType";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/azimuth>
    azimuth;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/baseFrequency>
    base_frequency: "baseFrequency";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/baseline-shift>
    baseline_shift: "baseline-shift";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/baseProfile>
    base_profile: "baseProfile";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/bbox>
    bbox;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/begin>
    begin;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/bias>
    bias;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/by>
    by;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/calcMode>
    calc_mode: "calcMode";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/cap-height>
    cap_height: "cap-height";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/class>
    class;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/clip>
    clip;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/clipPathUnits>
    clip_path_units: "clipPathUnits";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/clip-path>
    clip_path: "clip-path";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/clip-rule>
    clip_rule: "clip-rule";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/color>
    color;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/color-interpolation>
    color_interpolation: "color-interpolation";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/color-interpolation-filters>
    color_interpolation_filters: "color-interpolation-filters";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/color-profile>
    color_profile: "color-profile";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/color-rendering>
    color_rendering: "color-rendering";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/contentScriptType>
    content_script_type: "contentScriptType";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/contentStyleType>
    content_style_type: "contentStyleType";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/crossorigin>
    crossorigin;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/cursor>
    cursor;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/cx>
    cx;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/cy>
    cy;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/d>
    d;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/decelerate>
    decelerate;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/descent>
    descent;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/diffuseConstant>
    diffuse_constant: "diffuseConstant";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/direction>
    direction;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/display>
    display;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/divisor>
    divisor;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/dominant-baseline>
    dominant_baseline: "dominant-baseline";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/dur>
    dur;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/dx>
    dx;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/dy>
    dy;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/edgeMode>
    edge_mode: "edgeMode";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/elevation>
    elevation;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/enable-background>
    enable_background: "enable-background";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/end>
    end;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/exponent>
    exponent;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fill>
    fill;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fill-opacity>
    fill_opacity: "fill-opacity";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fill-rule>
    fill_rule: "fill-rule";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/filter>
    filter;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/filterRes>
    filterRes;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/filterUnits>
    filterUnits;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/flood-color>
    flood_color: "flood-color";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/flood-opacity>
    flood_opacity: "flood-opacity";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/font-family>
    font_family: "font-family";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/font-size>
    font_size: "font-size";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/font-size-adjust>
    font_size_adjust: "font-size-adjust";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/font-stretch>
    font_stretch: "font-stretch";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/font-style>
    font_style: "font-style";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/font-variant>
    font_variant: "font-variant";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/font-weight>
    font_weight: "font-weight";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/format>
    format;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/from>
    from;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fr>
    fr;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fx>
    fx;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fy>
    fy;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/g1>
    g1;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/g2>
    g2;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/glyph-name>
    glyph_name: "glyph-name";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/glyph-orientation-horizontal>
    glyph_orientation_horizontal: "glyph-orientation-horizontal";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/glyph-orientation-vertical>
    glyph_orientation_vertical: "glyph-orientation-vertical";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/glyphRef>
    glyph_ref: "glyphRef";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/gradientTransform>
    gradient_transform: "gradientTransform";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/gradientUnits>
    gradient_units: "gradientUnits";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/hanging>
    hanging;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/height>
    height;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/href>
    href;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/hreflang>
    hreflang;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/horiz-adv-x>
    horiz_adv_x: "horiz-adv-x";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/horiz-origin-x>
    horiz_origin_x: "horiz-origin-x";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/id>
    id;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/ideographic>
    ideographic;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/image-rendering>
    image_rendering: "image-rendering";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/_in>
    _in;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/in2>
    in2;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/intercept>
    intercept;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/k>
    k;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/k1>
    k1;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/k2>
    k2;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/k3>
    k3;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/k4>
    k4;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/kernelMatrix>
    kernel_matrix: "kernelMatrix";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/kernelUnitLength>
    kernel_unit_length: "kernelUnitLength";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/kerning>
    kerning;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/keyPoints>
    key_points: "keyPoints";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/keySplines>
    key_splines: "keySplines";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/keyTimes>
    key_times: "keyTimes";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/lang>
    lang;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/lengthAdjust>
    length_adjust: "lengthAdjust";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/letter-spacing>
    letter_spacing: "letter-spacing";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/lighting-color>
    lighting_color: "lighting-color";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/limitingConeAngle>
    limiting_cone_angle: "limitingConeAngle";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/local>
    local;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/marker-end>
    marker_end: "marker-end";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/marker-mid>
    marker_mid: "marker-mid";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/marker-start>
    marker_start: "marker-start";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/markerHeight>
    marker_height: "markerHeight";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/markerUnits>
    marker_units: "markerUnits";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/markerWidth>
    marker_width: "markerWidth";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/mask>
    mask;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/maskContentUnits>
    mask_content_units: "maskContentUnits";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/maskUnits>
    mask_units: "maskUnits";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/mathematical>
    mathematical;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/max>
    max;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/media>
    media;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/method>
    method;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/min>
    min;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/mode>
    mode;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/name>
    name;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/numOctaves>
    num_octaves: "numOctaves";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/offset>
    offset;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/opacity>
    opacity;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/operator>
    operator;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/order>
    order;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/orient>
    orient;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/orientation>
    orientation;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/origin>
    origin;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/overflow>
    overflow;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/overline-position>
    overline_position: "overline-position";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/overline-thickness>
    overline_thickness: "overline-thickness";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/panose-1>
    panose_1: "panose-1";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/paint-order>
    paint_order: "paint-order";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/path>
    path;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/pathLength>
    path_length: "pathLength";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/patternContentUnits>
    pattern_content_units: "patternContentUnits";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/patternTransform>
    pattern_transform: "patternTransform";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/patternUnits>
    pattern_units: "patternUnits";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/ping>
    ping;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/pointer-events>
    pointer_events: "pointer-events";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/points>
    points;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/pointsAtX>
    points_at_x: "pointsAtX";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/pointsAtY>
    points_at_y: "pointsAtY";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/pointsAtZ>
    points_at_z: "pointsAtZ";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/preserveAlpha>
    preserve_alpha: "preserveAlpha";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/preserveAspectRatio>
    preserve_aspect_ratio: "preserveAspectRatio";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/primitiveUnits>
    primitive_units: "primitiveUnits";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/r>
    r;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/radius>
    radius;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/referrerPolicy>
    referrer_policy: "referrerPolicy";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/refX>
    ref_x: "refX";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/refY>
    ref_y: "refY";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/rel>
    rel;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/rendering-intent>
    rendering_intent: "rendering-intent";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/repeatCount>
    repeat_count: "repeatCount";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/repeatDur>
    repeat_dur: "repeatDur";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/requiredExtensions>
    required_extensions: "requiredExtensions";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/requiredFeatures>
    required_features: "requiredFeatures";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/restart>
    restart;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/result>
    result;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/role>
    role;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/rotate>
    rotate;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/rx>
    rx;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/ry>
    ry;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/scale>
    scale;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/seed>
    seed;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/shape-rendering>
    shape_rendering: "shape-rendering";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/slope>
    slope;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/spacing>
    spacing;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/specularConstant>
    specular_constant: "specularConstant";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/specularExponent>
    specular_exponent: "specularExponent";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/speed>
    speed;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/spreadMethod>
    spread_method: "spreadMethod";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/startOffset>
    start_offset: "startOffset";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stdDeviation>
    std_deviation: "stdDeviation";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stemh>
    stemh;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stemv>
    stemv;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stitchTiles>
    stitch_tiles: "stitchTiles";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stop-color>
    stop_color: "stop-color";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stop-opacity>
    stop_opacity: "stop-opacity";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/strikethrough-position>
    strikethrough_position: "strikethrough-position";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/strikethrough-thickness>
    strikethrough_thickness: "strikethrough-thickness";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/string>
    string;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke>
    stroke;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-dasharray>
    stroke_dasharray: "stroke-dasharray";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-dashoffset>
    stroke_dashoffset: "stroke-dashoffset";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-linecap>
    stroke_linecap: "stroke-linecap";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-linejoin>
    stroke_linejoin: "stroke-linejoin";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-miterlimit>
    stroke_miterlimit: "stroke-miterlimit";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-opacity>
    stroke_opacity: "stroke-opacity";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke-width>
    stroke_width: "stroke-width";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/style>
    style;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/surfaceScale>
    surface_scale: "surfaceScale";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/systemLanguage>
    system_language: "systemLanguage";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/tabindex>
    tabindex;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/tableValues>
    table_values: "tableValues";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/target>
    target;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/targetX>
    target_x: "targetX";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/targetY>
    target_y: "targetY";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/text-anchor>
    text_anchor: "text-anchor";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/text-decoration>
    text_decoration: "text-decoration";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/text-rendering>
    text_rendering: "text-rendering";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/textLength>
    text_length: "textLength";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/to>
    to;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/transform>
    transform;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/transform-origin>
    transform_origin: "transform-origin";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/type>
    r#type: no-alias "type";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/u1>
    u1;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/u2>
    u2;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/underline-position>
    underline_position: "underline-position";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/underline-thickness>
    underline_thickness: "underline-thickness";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/unicode>
    unicode;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/unicode-bidi>
    unicode_bidi: "unicode-bidi";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/unicode-range>
    unicode_range: "unicode-range";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/units-per-em>
    units_per_em: "units-per-em";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/v-alphabetic>
    v_alphabetic: "v-alphabetic";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/v-hanging>
    v_hanging: "v-hanging";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/v-ideographic>
    v_ideographic: "v-ideographic";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/v-mathematical>
    v_mathematical: "v-mathematical";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/values>
    values;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/vector-effect>
    vector_effect: "vector-effect";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/version>
    version;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/vert-adv-y>
    vert_adv_y: "vert-adv-y";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/vert-origin-x>
    vert_origin_x: "vert-origin-x";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/vert-origin-y>
    vert_origin_y: "vert-origin-y";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/viewBox>
    view_box: "viewBox";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/viewTarget>
    view_target: "viewTarget";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/visibility>
    visibility;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/width>
    width;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/widths>
    widths;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/word-spacing>
    word_spacing: "word-spacing";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/writing-mode>
    writing_mode: "writing-mode";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/x>
    x;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/x-height>
    x_height: "x-height";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/x1>
    x1;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/x2>
    x2;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/xmlns>
    xmlns;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/xChannelSelector>
    x_channel_selector: "xChannelSelector";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/y>
    y;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/y1>
    y1;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/y2>
    y2;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/yChannelSelector>
    y_channel_selector: "yChannelSelector";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/z>
    z;

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/zoomAndPan>
    zoom_and_pan: "zoomAndPan";

}
