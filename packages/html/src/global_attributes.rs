#![allow(non_upper_case_globals)]

use dioxus_core::prelude::IntoAttributeValue;
use dioxus_core::HasAttributes;
use dioxus_html_internal_macro::impl_extension_attributes;

use crate::AttributeDiscription;

#[cfg(feature = "hot-reload-context")]
macro_rules! trait_method_mapping {
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
        $name:ident: $lit:literal, $ns:literal;
    ) => {
        if $matching == stringify!($name) {
            return Some(($lit, Some($ns)));
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
        $name:ident: $lit:literal, $ns:literal;
    ) => {
        if $matching == stringify!($lit) {
            return Some(stringify!($name));
        }
    };
}

macro_rules! trait_methods {
    (
        @base
        $(#[$trait_attr:meta])*
        $trait:ident;
        $fn:ident;
        $fn_html_to_rsx:ident;
        $(
            $(#[$attr:meta])*
            $name:ident $(: $($arg:literal),*)*;
        )+
    ) => {
        $(#[$trait_attr])*
        pub trait $trait {
            $(
                $(#[$attr])*
                const $name: AttributeDiscription = trait_methods! { $name $(: $($arg),*)*; };
            )*
        }

        #[cfg(feature = "hot-reload-context")]
        pub(crate) fn $fn(attr: &str) -> Option<(&'static str, Option<&'static str>)> {
            $(
                trait_method_mapping! {
                    attr;
                    $name$(: $($arg),*)*;
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
                    $name$(: $($arg),*)*;
                }
            )*
            None
        }

        impl_extension_attributes![GLOBAL $trait { $($name,)* }];
    };

    // Rename the incoming ident and apply a custom namespace
    ( $name:ident: $lit:literal, $ns:literal; ) => { ($lit, Some($ns), false) };

    // Rename the incoming ident
    ( $name:ident: $lit:literal; ) => { ($lit, None, false ) };

    // Don't rename the incoming ident
    ( $name:ident; ) => { (stringify!($name), None, false) };
}

trait_methods! {
    @base

    GlobalAttributes;
    map_global_attributes;
    map_html_global_attributes_to_rsx;

    /// Prevent the default action for this element.
    ///
    /// For more information, see the MDN docs:
    /// <https://developer.mozilla.org/en-US/docs/Web/API/Event/preventDefault>
    prevent_default: "dioxus-prevent-default";


    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/accesskey>
    accesskey: "accesskey";


    /// <https://developer.mozilla.org/en-US/docs/Web/HTML/Global_attributes/autocapitalize>
    autocapitalize: "autocapitalize";


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
    /// ## Example
    ///
    /// ### HTML:
    /// ```html
    /// <p class="note editorial">Above point sounds a bit obvious. Remove/rewrite?</p>
    /// ```
    ///
    /// ### CSS:
    /// ```css
    /// .note {
    ///     font-style: italic;
    ///     font-weight: bold;
    /// }
    ///
    /// .editorial {
    ///     background: rgb(255, 0, 0, .25);
    ///     padding: 10px;
    /// }
    /// ```
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
    align_content: "align-content", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/align-items>
    align_items: "align-items", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/align-self>
    align_self: "align-self", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/alignment-adjust>
    alignment_adjust: "alignment-adjust", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/alignment-baseline>
    alignment_baseline: "alignment-baseline", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/all>
    all: "all", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/alt>
    alt: "alt", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation>
    animation: "animation", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-delay>
    animation_delay: "animation-delay", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-direction>
    animation_direction: "animation-direction", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-duration>
    animation_duration: "animation-duration", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-fill-mode>
    animation_fill_mode: "animation-fill-mode", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-iteration-count>
    animation_iteration_count: "animation-iteration-count", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-name>
    animation_name: "animation-name", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-play-state>
    animation_play_state: "animation-play-state", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/animation-timing-function>
    animation_timing_function: "animation-timing-function", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/azimuth>
    azimuth: "azimuth", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/backdrop-filter>
    backdrop_filter: "backdrop-filter", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/backface-visibility>
    backface_visibility: "backface-visibility", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background>
    background: "background", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-attachment>
    background_attachment: "background-attachment", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-clip>
    background_clip: "background-clip", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-color>
    background_color: "background-color", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-image>
    background_image: "background-image", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-origin>
    background_origin: "background-origin", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-position>
    background_position: "background-position", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-repeat>
    background_repeat: "background-repeat", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-size>
    background_size: "background-size", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/background-blend-mode>
    background_blend_mode: "background-blend-mode", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/baseline-shift>
    baseline_shift: "baseline-shift", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/bleed>
    bleed: "bleed", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/bookmark-label>
    bookmark_label: "bookmark-label", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/bookmark-level>
    bookmark_level: "bookmark-level", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/bookmark-state>
    bookmark_state: "bookmark-state", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border>
    border: "border", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-color>
    border_color: "border-color", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-style>
    border_style: "border-style", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-width>
    border_width: "border-width", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom>
    border_bottom: "border-bottom", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom-color>
    border_bottom_color: "border-bottom-color", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom-style>
    border_bottom_style: "border-bottom-style", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom-width>
    border_bottom_width: "border-bottom-width", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-left>
    border_left: "border-left", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-left-color>
    border_left_color: "border-left-color", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-left-style>
    border_left_style: "border-left-style", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-left-width>
    border_left_width: "border-left-width", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-right>
    border_right: "border-right", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-right-color>
    border_right_color: "border-right-color", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-right-style>
    border_right_style: "border-right-style", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-right-width>
    border_right_width: "border-right-width", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top>
    border_top: "border-top", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top-color>
    border_top_color: "border-top-color", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top-style>
    border_top_style: "border-top-style", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top-width>
    border_top_width: "border-top-width", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-collapse>
    border_collapse: "border-collapse", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image>
    border_image: "border-image", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image-outset>
    border_image_outset: "border-image-outset", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image-repeat>
    border_image_repeat: "border-image-repeat", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image-slice>
    border_image_slice: "border-image-slice", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image-source>
    border_image_source: "border-image-source", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-image-width>
    border_image_width: "border-image-width", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-radius>
    border_radius: "border-radius", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom-left-radius>
    border_bottom_left_radius: "border-bottom-left-radius", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-bottom-right-radius>
    border_bottom_right_radius: "border-bottom-right-radius", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top-left-radius>
    border_top_left_radius: "border-top-left-radius", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-top-right-radius>
    border_top_right_radius: "border-top-right-radius", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/border-spacing>
    border_spacing: "border-spacing", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/bottom>
    bottom: "bottom", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/box-decoration-break>
    box_decoration_break: "box-decoration-break", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/box-shadow>
    box_shadow: "box-shadow", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/box-sizing>
    box_sizing: "box-sizing", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/box-snap>
    box_snap: "box-snap", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/break-after>
    break_after: "break-after", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/break-before>
    break_before: "break-before", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/break-inside>
    break_inside: "break-inside", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/buffered-rendering>
    buffered_rendering: "buffered-rendering", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/caption-side>
    caption_side: "caption-side", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/clear>
    clear: "clear", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/clear-side>
    clear_side: "clear-side", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/clip>
    clip: "clip", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/clip-path>
    clip_path: "clip-path", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/clip-rule>
    clip_rule: "clip-rule", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color>
    color: "color", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-adjust>
    color_adjust: "color-adjust", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-correction>
    color_correction: "color-correction", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-interpolation>
    color_interpolation: "color-interpolation", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-interpolation-filters>
    color_interpolation_filters: "color-interpolation-filters", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-profile>
    color_profile: "color-profile", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/color-rendering>
    color_rendering: "color-rendering", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-fill>
    column_fill: "column-fill", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-gap>
    column_gap: "column-gap", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-rule>
    column_rule: "column-rule", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-rule-color>
    column_rule_color: "column-rule-color", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-rule-style>
    column_rule_style: "column-rule-style", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-rule-width>
    column_rule_width: "column-rule-width", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-span>
    column_span: "column-span", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/columns>
    columns: "columns", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-count>
    column_count: "column-count", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/column-width>
    column_width: "column-width", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/contain>
    contain: "contain", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/content>
    content: "content", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/counter-increment>
    counter_increment: "counter-increment", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/counter-reset>
    counter_reset: "counter-reset", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/counter-set>
    counter_set: "counter-set", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/cue>
    cue: "cue", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/cue-after>
    cue_after: "cue-after", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/cue-before>
    cue_before: "cue-before", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/cursor>
    cursor: "cursor", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/direction>
    direction: "direction", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/display>
    display: "display", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/display-inside>
    display_inside: "display-inside", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/display-outside>
    display_outside: "display-outside", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/display-extras>
    display_extras: "display-extras", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/display-box>
    display_box: "display-box", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/dominant-baseline>
    dominant_baseline: "dominant-baseline", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/elevation>
    elevation: "elevation", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/empty-cells>
    empty_cells: "empty-cells", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/enable-background>
    enable_background: "enable-background", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/fill>
    fill: "fill", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/fill-opacity>
    fill_opacity: "fill-opacity", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/fill-rule>
    fill_rule: "fill-rule", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/filter>
    filter: "filter", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/float>
    float: "float", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/float-defer-column>
    float_defer_column: "float-defer-column", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/float-defer-page>
    float_defer_page: "float-defer-page", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/float-offset>
    float_offset: "float-offset", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/float-wrap>
    float_wrap: "float-wrap", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flow-into>
    flow_into: "flow-into", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flow-from>
    flow_from: "flow-from", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex>
    flex: "flex", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-basis>
    flex_basis: "flex-basis", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-grow>
    flex_grow: "flex-grow", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-shrink>
    flex_shrink: "flex-shrink", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-flow>
    flex_flow: "flex-flow", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-direction>
    flex_direction: "flex-direction", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flex-wrap>
    flex_wrap: "flex-wrap", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flood-color>
    flood_color: "flood-color", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/flood-opacity>
    flood_opacity: "flood-opacity", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font>
    font: "font", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-family>
    font_family: "font-family", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-size>
    font_size: "font-size", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-stretch>
    font_stretch: "font-stretch", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-style>
    font_style: "font-style", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-weight>
    font_weight: "font-weight", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-feature-settings>
    font_feature_settings: "font-feature-settings", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-kerning>
    font_kerning: "font-kerning", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-language-override>
    font_language_override: "font-language-override", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-size-adjust>
    font_size_adjust: "font-size-adjust", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-synthesis>
    font_synthesis: "font-synthesis", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant>
    font_variant: "font-variant", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-alternates>
    font_variant_alternates: "font-variant-alternates", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-caps>
    font_variant_caps: "font-variant-caps", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-east-asian>
    font_variant_east_asian: "font-variant-east-asian", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-ligatures>
    font_variant_ligatures: "font-variant-ligatures", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-numeric>
    font_variant_numeric: "font-variant-numeric", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/font-variant-position>
    font_variant_position: "font-variant-position", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/footnote-policy>
    footnote_policy: "footnote-policy", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/glyph-orientation-horizontal>
    glyph_orientation_horizontal: "glyph-orientation-horizontal", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/glyph-orientation-vertical>
    glyph_orientation_vertical: "glyph-orientation-vertical", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid>
    grid: "grid", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-auto-flow>
    grid_auto_flow: "grid-auto-flow", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-auto-columns>
    grid_auto_columns: "grid-auto-columns", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-auto-rows>
    grid_auto_rows: "grid-auto-rows", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-template>
    grid_template: "grid-template", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-template-areas>
    grid_template_areas: "grid-template-areas", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-template-columns>
    grid_template_columns: "grid-template-columns", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-template-rows>
    grid_template_rows: "grid-template-rows", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-area>
    grid_area: "grid-area", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-column>
    grid_column: "grid-column", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-column-start>
    grid_column_start: "grid-column-start", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-column-end>
    grid_column_end: "grid-column-end", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-row>
    grid_row: "grid-row", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-row-start>
    grid_row_start: "grid-row-start", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/grid-row-end>
    grid_row_end: "grid-row-end", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hanging-punctuation>
    hanging_punctuation: "hanging-punctuation", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/height>
    height: "height", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphenate-character>
    hyphenate_character: "hyphenate-character", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphenate-limit-chars>
    hyphenate_limit_chars: "hyphenate-limit-chars", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphenate-limit-last>
    hyphenate_limit_last: "hyphenate-limit-last", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphenate-limit-lines>
    hyphenate_limit_lines: "hyphenate-limit-lines", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphenate-limit-zone>
    hyphenate_limit_zone: "hyphenate-limit-zone", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/hyphens>
    hyphens: "hyphens", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/icon>
    icon: "icon", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/image-orientation>
    image_orientation: "image-orientation", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/image-resolution>
    image_resolution: "image-resolution", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/image-rendering>
    image_rendering: "image-rendering", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ime>
    ime: "ime", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ime-align>
    ime_align: "ime-align", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ime-mode>
    ime_mode: "ime-mode", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ime-offset>
    ime_offset: "ime-offset", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ime-width>
    ime_width: "ime-width", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/initial-letters>
    initial_letters: "initial-letters", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/inline-box-align>
    inline_box_align: "inline-box-align", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/isolation>
    isolation: "isolation", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/justify-content>
    justify_content: "justify-content", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/justify-items>
    justify_items: "justify-items", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/justify-self>
    justify_self: "justify-self", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/kerning>
    kerning: "kerning", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/left>
    left: "left", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/letter-spacing>
    letter_spacing: "letter-spacing", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/lighting-color>
    lighting_color: "lighting-color", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-box-contain>
    line_box_contain: "line-box-contain", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-break>
    line_break: "line-break", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-grid>
    line_grid: "line-grid", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-height>
    line_height: "line-height", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-slack>
    line_slack: "line-slack", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/line-snap>
    line_snap: "line-snap", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/list-style>
    list_style: "list-style", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/list-style-image>
    list_style_image: "list-style-image", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/list-style-position>
    list_style_position: "list-style-position", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/list-style-type>
    list_style_type: "list-style-type", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/margin>
    margin: "margin", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/margin-bottom>
    margin_bottom: "margin-bottom", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/margin-left>
    margin_left: "margin-left", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/margin-right>
    margin_right: "margin-right", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/margin-top>
    margin_top: "margin-top", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker>
    marker: "marker", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-end>
    marker_end: "marker-end", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-mid>
    marker_mid: "marker-mid", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-pattern>
    marker_pattern: "marker-pattern", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-segment>
    marker_segment: "marker-segment", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-start>
    marker_start: "marker-start", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-knockout-left>
    marker_knockout_left: "marker-knockout-left", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-knockout-right>
    marker_knockout_right: "marker-knockout-right", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marker-side>
    marker_side: "marker-side", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marks>
    marks: "marks", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marquee-direction>
    marquee_direction: "marquee-direction", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marquee-play-count>
    marquee_play_count: "marquee-play-count", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marquee-speed>
    marquee_speed: "marquee-speed", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/marquee-style>
    marquee_style: "marquee-style", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask>
    mask: "mask", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-image>
    mask_image: "mask-image", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-repeat>
    mask_repeat: "mask-repeat", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-position>
    mask_position: "mask-position", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-clip>
    mask_clip: "mask-clip", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-origin>
    mask_origin: "mask-origin", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-size>
    mask_size: "mask-size", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box>
    mask_box: "mask-box", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box-outset>
    mask_box_outset: "mask-box-outset", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box-repeat>
    mask_box_repeat: "mask-box-repeat", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box-slice>
    mask_box_slice: "mask-box-slice", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box-source>
    mask_box_source: "mask-box-source", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-box-width>
    mask_box_width: "mask-box-width", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mask-type>
    mask_type: "mask-type", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/max-height>
    max_height: "max-height", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/max-lines>
    max_lines: "max-lines", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/max-width>
    max_width: "max-width", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/min-height>
    min_height: "min-height", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/min-width>
    min_width: "min-width", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/mix-blend-mode>
    mix_blend_mode: "mix-blend-mode", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/nav-down>
    nav_down: "nav-down", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/nav-index>
    nav_index: "nav-index", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/nav-left>
    nav_left: "nav-left", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/nav-right>
    nav_right: "nav-right", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/nav-up>
    nav_up: "nav-up", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/object-fit>
    object_fit: "object-fit", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/object-position>
    object_position: "object-position", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/offset-after>
    offset_after: "offset-after", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/offset-before>
    offset_before: "offset-before", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/offset-end>
    offset_end: "offset-end", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/offset-start>
    offset_start: "offset-start", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/opacity>
    opacity: "opacity", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/order>
    order: "order", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/orphans>
    orphans: "orphans", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/outline>
    outline: "outline", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/outline-color>
    outline_color: "outline-color", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/outline-style>
    outline_style: "outline-style", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/outline-width>
    outline_width: "outline-width", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/outline-offset>
    outline_offset: "outline-offset", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/overflow>
    overflow: "overflow", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/overflow-x>
    overflow_x: "overflow-x", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/overflow-y>
    overflow_y: "overflow-y", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/overflow-style>
    overflow_style: "overflow-style", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/overflow-wrap>
    overflow_wrap: "overflow-wrap", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/padding>
    padding: "padding", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/padding-bottom>
    padding_bottom: "padding-bottom", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/padding-left>
    padding_left: "padding-left", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/padding-right>
    padding_right: "padding-right", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/padding-top>
    padding_top: "padding-top", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/page>
    page: "page", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/page-break-after>
    page_break_after: "page-break-after", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/page-break-before>
    page_break_before: "page-break-before", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/page-break-inside>
    page_break_inside: "page-break-inside", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/paint-order>
    paint_order: "paint-order", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pause>
    pause: "pause", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pause-after>
    pause_after: "pause-after", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pause-before>
    pause_before: "pause-before", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/perspective>
    perspective: "perspective", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/perspective-origin>
    perspective_origin: "perspective-origin", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pitch>
    pitch: "pitch", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pitch-range>
    pitch_range: "pitch-range", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/play-during>
    play_during: "play-during", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/pointer-events>
    pointer_events: "pointer-events", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/position>
    position: "position", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/quotes>
    quotes: "quotes", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/region-fragment>
    region_fragment: "region-fragment", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/resize>
    resize: "resize", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/rest>
    rest: "rest", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/rest-after>
    rest_after: "rest-after", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/rest-before>
    rest_before: "rest-before", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/richness>
    richness: "richness", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/right>
    right: "right", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ruby-align>
    ruby_align: "ruby-align", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ruby-merge>
    ruby_merge: "ruby-merge", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/ruby-position>
    ruby_position: "ruby-position", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-behavior>
    scroll_behavior: "scroll-behavior", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-snap-coordinate>
    scroll_snap_coordinate: "scroll-snap-coordinate", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-snap-destination>
    scroll_snap_destination: "scroll-snap-destination", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-snap-points-x>
    scroll_snap_points_x: "scroll-snap-points-x", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-snap-points-y>
    scroll_snap_points_y: "scroll-snap-points-y", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/scroll-snap-type>
    scroll_snap_type: "scroll-snap-type", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-image-threshold>
    shape_image_threshold: "shape-image-threshold", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-inside>
    shape_inside: "shape-inside", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-margin>
    shape_margin: "shape-margin", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-outside>
    shape_outside: "shape-outside", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-padding>
    shape_padding: "shape-padding", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/shape-rendering>
    shape_rendering: "shape-rendering", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/size>
    size: "size", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speak>
    speak: "speak", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speak-as>
    speak_as: "speak-as", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speak-header>
    speak_header: "speak-header", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speak-numeral>
    speak_numeral: "speak-numeral", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speak-punctuation>
    speak_punctuation: "speak-punctuation", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/speech-rate>
    speech_rate: "speech-rate", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stop-color>
    stop_color: "stop-color", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stop-opacity>
    stop_opacity: "stop-opacity", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stress>
    stress: "stress", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/string-set>
    string_set: "string-set", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke>
    stroke: "stroke", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-dasharray>
    stroke_dasharray: "stroke-dasharray", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-dashoffset>
    stroke_dashoffset: "stroke-dashoffset", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-linecap>
    stroke_linecap: "stroke-linecap", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-linejoin>
    stroke_linejoin: "stroke-linejoin", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-miterlimit>
    stroke_miterlimit: "stroke-miterlimit", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-opacity>
    stroke_opacity: "stroke-opacity", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/stroke-width>
    stroke_width: "stroke-width", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/tab-size>
    tab_size: "tab-size", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/table-layout>
    table_layout: "table-layout", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-align>
    text_align: "text-align", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-align-all>
    text_align_all: "text-align-all", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-align-last>
    text_align_last: "text-align-last", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-anchor>
    text_anchor: "text-anchor", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-combine-upright>
    text_combine_upright: "text-combine-upright", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration>
    text_decoration: "text-decoration", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration-color>
    text_decoration_color: "text-decoration-color", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration-line>
    text_decoration_line: "text-decoration-line", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration-style>
    text_decoration_style: "text-decoration-style", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-decoration-skip>
    text_decoration_skip: "text-decoration-skip", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-emphasis>
    text_emphasis: "text-emphasis", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-emphasis-color>
    text_emphasis_color: "text-emphasis-color", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-emphasis-style>
    text_emphasis_style: "text-emphasis-style", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-emphasis-position>
    text_emphasis_position: "text-emphasis-position", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-emphasis-skip>
    text_emphasis_skip: "text-emphasis-skip", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-height>
    text_height: "text-height", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-indent>
    text_indent: "text-indent", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-justify>
    text_justify: "text-justify", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-orientation>
    text_orientation: "text-orientation", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-overflow>
    text_overflow: "text-overflow", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-rendering>
    text_rendering: "text-rendering", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-shadow>
    text_shadow: "text-shadow", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-size-adjust>
    text_size_adjust: "text-size-adjust", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-space-collapse>
    text_space_collapse: "text-space-collapse", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-spacing>
    text_spacing: "text-spacing", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-transform>
    text_transform: "text-transform", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-underline-position>
    text_underline_position: "text-underline-position", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/text-wrap>
    text_wrap: "text-wrap", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/top>
    top: "top", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/touch-action>
    touch_action: "touch-action", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transform>
    transform: "transform", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transform-box>
    transform_box: "transform-box", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transform-origin>
    transform_origin: "transform-origin", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transform-style>
    transform_style: "transform-style", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transition>
    transition: "transition", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transition-delay>
    transition_delay: "transition-delay", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transition-duration>
    transition_duration: "transition-duration", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transition-property>
    transition_property: "transition-property", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/unicode-bidi>
    unicode_bidi: "unicode-bidi", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/vector-effect>
    vector_effect: "vector-effect", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/vertical-align>
    vertical_align: "vertical-align", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/visibility>
    visibility: "visibility", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-balance>
    voice_balance: "voice-balance", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-duration>
    voice_duration: "voice-duration", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-family>
    voice_family: "voice-family", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-pitch>
    voice_pitch: "voice-pitch", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-range>
    voice_range: "voice-range", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-rate>
    voice_rate: "voice-rate", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-stress>
    voice_stress: "voice-stress", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/voice-volumn>
    voice_volumn: "voice-volumn", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/volume>
    volume: "volume", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/white-space>
    white_space: "white-space", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/widows>
    widows: "widows", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/width>
    width: "width", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/will-change>
    will_change: "will-change", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/word-break>
    word_break: "word-break", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/word-spacing>
    word_spacing: "word-spacing", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/word-wrap>
    word_wrap: "word-wrap", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/wrap-flow>
    wrap_flow: "wrap-flow", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/wrap-through>
    wrap_through: "wrap-through", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/writing-mode>
    writing_mode: "writing-mode", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/gap>
    gap: "gap", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/list-style-type>
    list_styler_type: "list-style-type", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/row-gap>
    row_gap: "row-gap", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/transition-timing-function>
    transition_timing_function: "transition-timing-function", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/user-select>
    user_select: "user-select", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/-webkit-user-select>
    webkit_user_select: "-webkit-user-select", "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/CSS/z-index>
    z_index : "z-index", "style";

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

trait_methods! {
    @base
    SvgAttributes;
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
    accumulate: "accumulate";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/additive>
    additive: "additive";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/alignment-baseline>
    alignment_baseline: "alignment-baseline";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/alphabetic>
    alphabetic: "alphabetic";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/amplitude>
    amplitude: "amplitude";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/arabic-form>
    arabic_form: "arabic-form";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/ascent>
    ascent: "ascent";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/attributeName>
    attribute_name: "attributeName";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/attributeType>
    attribute_type: "attributeType";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/azimuth>
    azimuth: "azimuth";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/baseFrequency>
    base_frequency: "baseFrequency";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/baseline-shift>
    baseline_shift: "baseline-shift";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/baseProfile>
    base_profile: "baseProfile";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/bbox>
    bbox: "bbox";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/begin>
    begin: "begin";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/bias>
    bias: "bias";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/by>
    by: "by";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/calcMode>
    calc_mode: "calcMode";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/cap-height>
    cap_height: "cap-height";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/class>
    class: "class";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/clip>
    clip: "clip";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/clipPathUnits>
    clip_path_units: "clipPathUnits";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/clip-path>
    clip_path: "clip-path";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/clip-rule>
    clip_rule: "clip-rule";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/color>
    color: "color";

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
    crossorigin: "crossorigin";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/cursor>
    cursor: "cursor";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/cx>
    cx: "cx";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/cy>
    cy: "cy";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/d>
    d: "d";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/decelerate>
    decelerate: "decelerate";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/descent>
    descent: "descent";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/diffuseConstant>
    diffuse_constant: "diffuseConstant";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/direction>
    direction: "direction";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/display>
    display: "display";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/divisor>
    divisor: "divisor";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/dominant-baseline>
    dominant_baseline: "dominant-baseline";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/dur>
    dur: "dur";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/dx>
    dx: "dx";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/dy>
    dy: "dy";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/edgeMode>
    edge_mode: "edgeMode";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/elevation>
    elevation: "elevation";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/enable-background>
    enable_background: "enable-background";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/end>
    end: "end";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/exponent>
    exponent: "exponent";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fill>
    fill: "fill";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fill-opacity>
    fill_opacity: "fill-opacity";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fill-rule>
    fill_rule: "fill-rule";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/filter>
    filter: "filter";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/filterRes>
    filterRes: "filterRes";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/filterUnits>
    filterUnits: "filterUnits";

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
    format: "format";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/from>
    from: "from";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fr>
    fr: "fr";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fx>
    fx: "fx";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/fy>
    fy: "fy";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/g1>
    g1: "g1";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/g2>
    g2: "g2";

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
    hanging: "hanging";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/height>
    height: "height";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/href>
    href: "href";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/hreflang>
    hreflang: "hreflang";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/horiz-adv-x>
    horiz_adv_x: "horiz-adv-x";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/horiz-origin-x>
    horiz_origin_x: "horiz-origin-x";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/id>
    id: "id";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/ideographic>
    ideographic: "ideographic";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/image-rendering>
    image_rendering: "image-rendering";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/_in>
    _in: "_in";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/in2>
    in2: "in2";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/intercept>
    intercept: "intercept";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/k>
    k: "k";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/k1>
    k1: "k1";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/k2>
    k2: "k2";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/k3>
    k3: "k3";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/k4>
    k4: "k4";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/kernelMatrix>
    kernel_matrix: "kernelMatrix";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/kernelUnitLength>
    kernel_unit_length: "kernelUnitLength";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/kerning>
    kerning: "kerning";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/keyPoints>
    key_points: "keyPoints";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/keySplines>
    key_splines: "keySplines";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/keyTimes>
    key_times: "keyTimes";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/lang>
    lang: "lang";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/lengthAdjust>
    length_adjust: "lengthAdjust";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/letter-spacing>
    letter_spacing: "letter-spacing";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/lighting-color>
    lighting_color: "lighting-color";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/limitingConeAngle>
    limiting_cone_angle: "limitingConeAngle";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/local>
    local: "local";

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
    mask: "mask";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/maskContentUnits>
    mask_content_units: "maskContentUnits";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/maskUnits>
    mask_units: "maskUnits";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/mathematical>
    mathematical: "mathematical";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/max>
    max: "max";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/media>
    media: "media";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/method>
    method: "method";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/min>
    min: "min";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/mode>
    mode: "mode";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/name>
    name: "name";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/numOctaves>
    num_octaves: "numOctaves";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/offset>
    offset: "offset";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/opacity>
    opacity: "opacity";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/operator>
    operator: "operator";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/order>
    order: "order";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/orient>
    orient: "orient";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/orientation>
    orientation: "orientation";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/origin>
    origin: "origin";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/overflow>
    overflow: "overflow";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/overline-position>
    overline_position: "overline-position";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/overline-thickness>
    overline_thickness: "overline-thickness";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/panose-1>
    panose_1: "panose-1";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/paint-order>
    paint_order: "paint-order";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/path>
    path: "path";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/pathLength>
    path_length: "pathLength";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/patternContentUnits>
    pattern_content_units: "patternContentUnits";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/patternTransform>
    pattern_transform: "patternTransform";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/patternUnits>
    pattern_units: "patternUnits";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/ping>
    ping: "ping";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/pointer-events>
    pointer_events: "pointer-events";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/points>
    points: "points";

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
    r: "r";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/radius>
    radius: "radius";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/referrerPolicy>
    referrer_policy: "referrerPolicy";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/refX>
    ref_x: "refX";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/refY>
    ref_y: "refY";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/rel>
    rel: "rel";

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
    restart: "restart";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/result>
    result: "result";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/role>
    role: "role";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/rotate>
    rotate: "rotate";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/rx>
    rx: "rx";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/ry>
    ry: "ry";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/scale>
    scale: "scale";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/seed>
    seed: "seed";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/shape-rendering>
    shape_rendering: "shape-rendering";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/slope>
    slope: "slope";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/spacing>
    spacing: "spacing";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/specularConstant>
    specular_constant: "specularConstant";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/specularExponent>
    specular_exponent: "specularExponent";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/speed>
    speed: "speed";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/spreadMethod>
    spread_method: "spreadMethod";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/startOffset>
    start_offset: "startOffset";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stdDeviation>
    std_deviation: "stdDeviation";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stemh>
    stemh: "stemh";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stemv>
    stemv: "stemv";

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
    string: "string";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/stroke>
    stroke: "stroke";

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
    style: "style";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/surfaceScale>
    surface_scale: "surfaceScale";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/systemLanguage>
    system_language: "systemLanguage";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/tabindex>
    tabindex: "tabindex";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/tableValues>
    table_values: "tableValues";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/target>
    target: "target";

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
    to: "to";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/transform>
    transform: "transform";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/transform-origin>
    transform_origin: "transform-origin";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/_type>
    r#type: "_type";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/u1>
    u1: "u1";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/u2>
    u2: "u2";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/underline-position>
    underline_position: "underline-position";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/underline-thickness>
    underline_thickness: "underline-thickness";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/unicode>
    unicode: "unicode";

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
    values: "values";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/vector-effect>
    vector_effect: "vector-effect";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/version>
    version: "version";

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
    visibility: "visibility";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/width>
    width: "width";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/widths>
    widths: "widths";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/word-spacing>
    word_spacing: "word-spacing";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/writing-mode>
    writing_mode: "writing-mode";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/x>
    x: "x";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/x-height>
    x_height: "x-height";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/x1>
    x1: "x1";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/x2>
    x2: "x2";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/xmlns>
    xmlns: "xmlns";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/xChannelSelector>
    x_channel_selector: "xChannelSelector";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/y>
    y: "y";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/y1>
    y1: "y1";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/y2>
    y2: "y2";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/yChannelSelector>
    y_channel_selector: "yChannelSelector";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/z>
    z: "z";

    /// <https://developer.mozilla.org/en-US/docs/Web/SVG/Attribute/zoomAndPan>
    zoom_and_pan: "zoomAndPan";

}
