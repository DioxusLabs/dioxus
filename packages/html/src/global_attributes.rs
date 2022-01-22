use dioxus_core::*;
use std::fmt::Arguments;

macro_rules! no_namespace_trait_methods {
    (
        $(
            $(#[$attr:meta])*
            $name:ident;
        )*
    ) => {
        $(
            $(#[$attr])*
            fn $name<'a>(&self, cx: NodeFactory<'a>, val: Arguments) -> Attribute<'a> {
                cx.attr(stringify!($name), val, None, false)
            }
        )*
    };
}
macro_rules! style_trait_methods {
    (
        $(
            $(#[$attr:meta])*
            $name:ident: $lit:literal,
        )*
    ) => {
        $(
            #[inline]
            $(#[$attr])*
            fn $name<'a>(&self, cx: NodeFactory<'a>, val: Arguments) -> Attribute<'a> {
                cx.attr($lit, val, Some("style"), false)
            }
        )*
    };
}
macro_rules! aria_trait_methods {
    (
        $(
            $(#[$attr:meta])*
            $name:ident: $lit:literal,
        )*
    ) => {
        $(
            $(#[$attr])*
            fn $name<'a>(&self, cx: NodeFactory<'a>, val: Arguments) -> Attribute<'a> {
                cx.attr($lit, val, None, false)
            }
        )*
    };
}

pub trait GlobalAttributes {
    fn prevent_default<'a>(&self, cx: NodeFactory<'a>, val: Arguments) -> Attribute<'a> {
        cx.attr("dioxus-prevent-default", val, None, false)
    }

    no_namespace_trait_methods! {
        accesskey;

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
        class;
        contenteditable;
        data;
        dir;
        draggable;
        hidden;
        id;
        lang;
        spellcheck;
        style;
        tabindex;
        title;
        translate;

        role;

        /// dangerous_inner_html is Dioxus's replacement for using innerHTML in the browser DOM. In general, setting
        /// HTML from code is risky because it’s easy to inadvertently expose your users to a cross-site scripting (XSS)
        /// attack. So, you can set HTML directly from Dioxus, but you have to type out dangerous_inner_html to remind
        /// yourself that it’s dangerous
        dangerous_inner_html;
    }

    // This macro creates an explicit method call for each of the style attributes.
    //
    // The left token specifies the name of the attribute in the rsx! macro, and the right string literal specifies the
    // actual name of the attribute generated.
    //
    // This roughly follows the html spec
    style_trait_methods! {
        /// Specifies the alignment of flexible container's items within the flex container.
        align_content: "align-content",

        /// Specifies the default alignment for items within the flex container.
        align_items: "align-items",

        /// Specifies the alignment for selected items within the flex container.
        align_self: "align-self",

        /// Specifies the keyframe_based animations.
        animation: "animation",

        /// Specifies when the animation will start.
        animation_delay: "animation-delay",

        /// Specifies whether the animation should play in reverse on alternate cycles or not.
        animation_direction: "animation-direction",

        /// Specifies the number of seconds or milliseconds an animation should take to complete one cycle
        animation_duration: "animation-duration",

        /// Specifies how a CSS animation should apply styles to its target before and after it is executing
        animation_fill_mode: "animation-fill-mode",

        /// Specifies the number of times an animation cycle should be played before stopping.
        animation_iteration_count: "animation-iteration-count",

        /// Specifies the name of @keyframes defined animations that should be applied to the selected element
        animation_name: "animation-name",

        /// Specifies whether the animation is running or paused.
        animation_play_state: "animation-play-state",

        /// Specifies how a CSS animation should progress over the duration of each cycle.
        animation_timing_function: "animation-timing-function",

        /// Specifies whether or not the "back" side of a transformed element is visible when facing the user.
        backface_visibility: "backface-visibility",

        /// Defines a variety of background properties within one declaration.
        background: "background",

        /// Specify whether the background image is fixed in the viewport or scrolls.
        background_attachment: "background-attachment",

        /// Specifies the painting area of the background.
        background_clip: "background-clip",

        /// Defines an element's background color.
        background_color: "background-color",

        /// Defines an element's background image.
        background_image: "background-image",

        /// Specifies the positioning area of the background images.
        background_origin: "background-origin",

        /// Defines the origin of a background image.
        background_position: "background-position",

        /// Specify whether/how the background image is tiled.
        background_repeat: "background-repeat",

        /// Specifies the size of the background images.
        background_size: "background-size",

        /// Sets the width, style, and color for all four sides of an element's border.
        border: "border",

        /// Sets the width, style, and color of the bottom border of an element.
        border_bottom: "border-bottom",

        /// Sets the color of the bottom border of an element.
        border_bottom_color: "border-bottom-color",

        /// Defines the shape of the bottom_left border corner of an element.
        border_bottom_left_radius: "border-bottom-left-radius",

        /// Defines the shape of the bottom_right border corner of an element.
        border_bottom_right_radius: "border-bottom-right-radius",

        /// Sets the style of the bottom border of an element.
        border_bottom_style: "border-bottom-style",

        /// Sets the width of the bottom border of an element.
        border_bottom_width: "border-bottom-width",

        /// Specifies whether table cell borders are connected or separated.
        border_collapse: "border-collapse",

        /// Sets the color of the border on all the four sides of an element.
        border_color: "border-color",

        /// Specifies how an image is to be used in place of the border styles.
        border_image: "border-image",

        /// Specifies the amount by which the border image area extends beyond the border box.
        border_image_outset: "border-image-outset",

        /// Specifies whether the image_border should be repeated, rounded or stretched.
        border_image_repeat: "border-image-repeat",

        /// Specifies the inward offsets of the image_border.
        border_image_slice: "border-image-slice",

        /// Specifies the location of the image to be used as a border.
        border_image_source: "border-image-source",

        /// Specifies the width of the image_border.
        border_image_width: "border-image-width",

        /// Sets the width, style, and color of the left border of an element.
        border_left: "border-left",

        /// Sets the color of the left border of an element.
        border_left_color: "border-left-color",

        /// Sets the style of the left border of an element.
        border_left_style: "border-left-style",

        /// Sets the width of the left border of an element.
        border_left_width: "border-left-width",

        /// Defines the shape of the border corners of an element.
        border_radius: "border-radius",

        /// Sets the width, style, and color of the right border of an element.
        border_right: "border-right",

        /// Sets the color of the right border of an element.
        border_right_color: "border-right-color",

        /// Sets the style of the right border of an element.
        border_right_style: "border-right-style",

        /// Sets the width of the right border of an element.
        border_right_width: "border-right-width",

        /// Sets the spacing between the borders of adjacent table cells.
        border_spacing: "border-spacing",

        /// Sets the style of the border on all the four sides of an element.
        border_style: "border-style",

        /// Sets the width, style, and color of the top border of an element.
        border_top: "border-top",

        /// Sets the color of the top border of an element.
        border_top_color: "border-top-color",

        /// Defines the shape of the top_left border corner of an element.
        border_top_left_radius: "border-top-left-radius",

        /// Defines the shape of the top_right border corner of an element.
        border_top_right_radius: "border-top-right-radius",

        /// Sets the style of the top border of an element.
        border_top_style: "border-top-style",

        /// Sets the width of the top border of an element.
        border_top_width: "border-top-width",

        /// Sets the width of the border on all the four sides of an element.
        border_width: "border-width",

        /// Specify the location of the bottom edge of the positioned element.
        bottom: "bottom",

        /// Applies one or more drop_shadows to the element's box.
        box_shadow: "box-shadow",

        /// Alter the default CSS box model.
        box_sizing: "box-sizing",

        /// Specify the position of table's caption.
        caption_side: "caption-side",

        /// Specifies the placement of an element in relation to floating elements.
        clear: "clear",

        /// Defines the clipping region.
        clip: "clip",

        /// Specify the color of the text of an element.
        color: "color",

        /// Specifies the number of columns in a multi_column element.
        column_count: "column-count",

        /// Specifies how columns will be filled.
        column_fill: "column-fill",

        /// Specifies the gap between the columns in a multi_column element.
        column_gap: "column-gap",

        /// Specifies a straight line, or "rule", to be drawn between each column in a multi_column element.
        column_rule: "column-rule",

        /// Specifies the color of the rules drawn between columns in a multi_column layout.
        column_rule_color: "column-rule-color",

        /// Specifies the style of the rule drawn between the columns in a multi_column layout.
        column_rule_style: "column-rule-style",

        /// Specifies the width of the rule drawn between the columns in a multi_column layout.
        column_rule_width: "column-rule-width",

        /// Specifies how many columns an element spans across in a multi_column layout.
        column_span: "column-span",

        /// Specifies the optimal width of the columns in a multi_column element.
        column_width: "column-width",

        /// A shorthand property for setting column_width and column_count properties.
        columns: "columns",

        /// Inserts generated content.
        content: "content",

        /// Increments one or more counter values.
        counter_increment: "counter-increment",

        /// Creates or resets one or more counters.
        counter_reset: "counter-reset",

        /// Specify the type of cursor.
        cursor: "cursor",

        /// Define the text direction/writing direction.
        direction: "direction",

        /// Specifies how an element is displayed onscreen.
        display: "display",

        /// Show or hide borders and backgrounds of empty table cells.
        empty_cells: "empty-cells",

        /// Specifies the components of a flexible length.
        flex: "flex",

        /// Specifies the initial main size of the flex item.
        flex_basis: "flex-basis",

        /// Specifies the direction of the flexible items.
        flex_direction: "flex-direction",

        /// A shorthand property for the flex_direction and the flex_wrap properties.
        flex_flow: "flex-flow",

        /// Specifies how the flex item will grow relative to the other items inside the flex container.
        flex_grow: "flex-grow",

        /// Specifies how the flex item will shrink relative to the other items inside the flex container
        flex_shrink: "flex-shrink",

        /// Specifies whether the flexible items should wrap or not.
        flex_wrap: "flex-wrap",

        /// Specifies whether or not a box should float.
        float: "float",

        /// Defines a variety of font properties within one declaration.
        font: "font",

        /// Defines a list of fonts for element.
        font_family: "font-family",

        /// Defines the font size for the text.
        font_size: "font-size",

        /// Preserves the readability of text when font fallback occurs.
        font_size_adjust: "font-size-adjust",

        /// Selects a normal, condensed, or expanded face from a font.
        font_stretch: "font-stretch",

        /// Defines the font style for the text.
        font_style: "font-style",

        /// Specify the font variant.
        font_variant: "font-variant",

        /// Specify the font weight of the text.
        font_weight: "font-weight",

        /// Sets gaps (gutters) between rows and columns. Shorthand for row_gap and column_gap.
        gap: "gap",

        /// Specify the height of an element.
        height: "height",

        /// Specifies how flex items are aligned along the main axis of the flex container after any flexible lengths and auto margins have been resolved.
        justify_content: "justify-content",

        /// Specify the location of the left edge of the positioned element.
        left: "left",

        /// Sets the extra spacing between letters.
        letter_spacing: "letter-spacing",

        /// Sets the height between lines of text.
        line_height: "line-height",

        /// Defines the display style for a list and list elements.
        list_style: "list-style",

        /// Specifies the image to be used as a list_item marker.
        list_style_image: "list-style-image",

        /// Specifies the position of the list_item marker.
        list_style_position: "list-style-position",

        /// Specifies the marker style for a list_item.
        list_styler_type: "list-style-type",

        /// Sets the margin on all four sides of the element.
        margin: "margin",

        /// Sets the bottom margin of the element.
        margin_bottom: "margin-bottom",

        /// Sets the left margin of the element.
        margin_left: "margin-left",

        /// Sets the right margin of the element.
        margin_right: "margin-right",

        /// Sets the top margin of the element.
        margin_top: "margin-top",

        /// Specify the maximum height of an element.
        max_height: "max-height",

        /// Specify the maximum width of an element.
        max_width: "max-width",

        /// Specify the minimum height of an element.
        min_height: "min-height",

        /// Specify the minimum width of an element.
        min_width: "min-width",

        /// Specifies the transparency of an element.
        opacity: "opacity",

        /// Specifies the order in which a flex items are displayed and laid out within a flex container.
        order: "order",

        /// Sets the width, style, and color for all four sides of an element's outline.
        outline: "outline",

        /// Sets the color of the outline.
        outline_color: "outline-color",

        /// Set the space between an outline and the border edge of an element.
        outline_offset: "outline-offset",

        /// Sets a style for an outline.
        outline_style: "outline-style",

        /// Sets the width of the outline.
        outline_width: "outline-width",

        /// Specifies the treatment of content that overflows the element's box.
        overflow: "overflow",

        /// Specifies the treatment of content that overflows the element's box horizontally.
        overflow_x: "overflow-x",

        /// Specifies the treatment of content that overflows the element's box vertically.
        overflow_y: "overflow-y",

        /// Sets the padding on all four sides of the element.
        padding: "padding",

        /// Sets the padding to the bottom side of an element.
        padding_bottom: "padding-bottom",

        /// Sets the padding to the left side of an element.
        padding_left: "padding-left",

        /// Sets the padding to the right side of an element.
        padding_right: "padding-right",

        /// Sets the padding to the top side of an element.
        padding_top: "padding-top",

        /// Insert a page breaks after an element.
        page_break_after: "page-break-after",

        /// Insert a page breaks before an element.
        page_break_before: "page-break-before",

        /// Insert a page breaks inside an element.
        page_break_inside: "page-break-inside",

        /// Defines the perspective from which all child elements of the object are viewed.
        perspective: "perspective",

        /// Defines the origin (the vanishing point for the 3D space) for the perspective property.
        perspective_origin: "perspective-origin",

        /// Specifies how an element is positioned.
        position: "position",

        /// The pointer-events CSS property sets under what circumstances (if any) a particular graphic element can
        /// become the target of pointer events.
        ///
        /// MDN: [`pointer_events`](https://developer.mozilla.org/en-US/docs/Web/CSS/pointer-events)
        pointer_events: "pointer-events",

        /// Specifies quotation marks for embedded quotations.
        quotes: "quotes",

        /// Specifies whether or not an element is resizable by the user.
        resize: "resize",

        /// Specify the location of the right edge of the positioned element.
        right: "right",

        /// Specifies the gap between the rows in a multi_column element.
        row_gap: "row-gap",

        /// Specifies the length of the tab character.
        tab_size: "tab-size",

        /// Specifies a table layout algorithm.
        table_layout: "table-layout",

        /// Sets the horizontal alignment of inline content.
        text_align: "text-align",

        /// Specifies how the last line of a block or a line right before a forced line break is aligned when  is justify.",
        text_align_last: "text-align-last",

        /// Specifies the decoration added to text.
        text_decoration: "text-decoration",

        /// Specifies the color of the text_decoration_line.
        text_decoration_color: "text-decoration-color",

        /// Specifies what kind of line decorations are added to the element.
        text_decoration_line: "text-decoration-line",

        /// Specifies the style of the lines specified by the text_decoration_line property
        text_decoration_style: "text-decoration-style",

        /// Indent the first line of text.
        text_indent: "text-indent",

        /// Specifies the justification method to use when the text_align property is set to justify.
        text_justify: "text-justify",

        /// Specifies how the text content will be displayed, when it overflows the block containers.
        text_overflow: "text-overflow",

        /// Applies one or more shadows to the text content of an element.
        text_shadow: "text-shadow",

        /// Transforms the case of the text.
        text_transform: "text-transform",

        /// Specify the location of the top edge of the positioned element.
        top: "top",

        /// Applies a 2D or 3D transformation to an element.
        transform: "transform",

        /// Defines the origin of transformation for an element.
        transform_origin: "transform-origin",

        /// Specifies how nested elements are rendered in 3D space.
        transform_style: "transform-style",

        /// Defines the transition between two states of an element.
        transition: "transition",

        /// Specifies when the transition effect will start.
        transition_delay: "transition-delay",

        /// Specifies the number of seconds or milliseconds a transition effect should take to complete.
        transition_duration: "transition-duration",

        /// Specifies the names of the CSS properties to which a transition effect should be applied.
        transition_property: "transition-property",

        /// Specifies the speed curve of the transition effect.
        transition_timing_function: "transition-timing-function",

        /// Sets the vertical positioning of an element relative to the current text baseline.
        vertical_align: "vertical-align",

        /// Specifies whether or not an element is visible.
        visibility: "visibility",

        /// Specifies how white space inside the element is handled.
        white_space: "white-space",

        /// Specify the width of an element.
        width: "width",

        /// Specifies how to break lines within words.
        word_break: "word-break",

        /// Sets the spacing between words.
        word_spacing: "word-spacing",

        /// Specifies whether to break words when the content overflows the boundaries of its container.
        word_wrap: "word-wrap",

        /// Specifies a layering or stacking order for positioned elements.
        z_index	: "z-index",

    }
    aria_trait_methods! {
        aria_current: "aria-current",
        aria_details: "aria-details",
        aria_disabled: "aria-disabled",
        aria_hidden: "aria-hidden",
        aria_invalid: "aria-invalid",
        aria_keyshortcuts: "aria-keyshortcuts",
        aria_label: "aria-label",
        aria_roledescription: "aria-roledescription",
        // Widget Attributes
        aria_autocomplete: "aria-autocomplete",
        aria_checked: "aria-checked",
        aria_expanded: "aria-expanded",
        aria_haspopup: "aria-haspopup",
        aria_level: "aria-level",
        aria_modal: "aria-modal",
        aria_multiline: "aria-multiline",
        aria_multiselectable: "aria-multiselectable",
        aria_orientation: "aria-orientation",
        aria_placeholder: "aria-placeholder",
        aria_pressed: "aria-pressed",
        aria_readonly: "aria-readonly",
        aria_required: "aria-required",
        aria_selected: "aria-selected",
        aria_sort: "aria-sort",
        aria_valuemax: "aria-valuemax",
        aria_valuemin: "aria-valuemin",
        aria_valuenow: "aria-valuenow",
        aria_valuetext: "aria-valuetext",
        // Live Region Attributes
        aria_atomic: "aria-atomic",
        aria_busy: "aria-busy",
        aria_live: "aria-live",
        aria_relevant: "aria-relevant",

        aria_dropeffect: "aria-dropeffect",
        aria_grabbed: "aria-grabbed",
        // Relationship Attributes
        aria_activedescendant: "aria-activedescendant",
        aria_colcount: "aria-colcount",
        aria_colindex: "aria-colindex",
        aria_colspan: "aria-colspan",
        aria_controls: "aria-controls",
        aria_describedby: "aria-describedby",
        aria_errormessage: "aria-errormessage",
        aria_flowto: "aria-flowto",
        aria_labelledby: "aria-labelledby",
        aria_owns: "aria-owns",
        aria_posinset: "aria-posinset",
        aria_rowcount: "aria-rowcount",
        aria_rowindex: "aria-rowindex",
        aria_rowspan: "aria-rowspan",
        aria_setsize: "aria-setsize",
    }
}

pub trait SvgAttributes {
    aria_trait_methods! {
        accent_height: "accent-height",
        accumulate: "accumulate",
        additive: "additive",
        alignment_baseline: "alignment-baseline",
        alphabetic: "alphabetic",
        amplitude: "amplitude",
        arabic_form: "arabic-form",
        ascent: "ascent",
        attributeName: "attributeName",
        attributeType: "attributeType",
        azimuth: "azimuth",
        baseFrequency: "baseFrequency",
        baseline_shift: "baseline-shift",
        baseProfile: "baseProfile",
        bbox: "bbox",
        begin: "begin",
        bias: "bias",
        by: "by",
        calcMode: "calcMode",
        cap_height: "cap-height",
        class: "class",
        clip: "clip",
        clipPathUnits: "clipPathUnits",
        clip_path: "clip-path",
        clip_rule: "clip-rule",
        color: "color",
        color_interpolation: "color-interpolation",
        color_interpolation_filters: "color-interpolation-filters",
        color_profile: "color-profile",
        color_rendering: "color-rendering",
        contentScriptType: "contentScriptType",
        contentStyleType: "contentStyleType",
        crossorigin: "crossorigin",
        cursor: "cursor",
        cx: "cx",
        cy: "cy",
        d: "d",
        decelerate: "decelerate",
        descent: "descent",
        diffuseConstant: "diffuseConstant",
        direction: "direction",
        display: "display",
        divisor: "divisor",
        dominant_baseline: "dominant-baseline",
        dur: "dur",
        dx: "dx",
        dy: "dy",
        edgeMode: "edgeMode",
        elevation: "elevation",
        enable_background: "enable-background",
        end: "end",
        exponent: "exponent",
        fill: "fill",
        fill_opacity: "fill-opacity",
        fill_rule: "fill-rule",
        filter: "filter",
        filterRes: "filterRes",
        filterUnits: "filterUnits",
        flood_color: "flood-color",
        flood_opacity: "flood-opacity",
        font_family: "font-family",
        font_size: "font-size",
        font_size_adjust: "font-size-adjust",
        font_stretch: "font-stretch",
        font_style: "font-style",
        font_variant: "font-variant",
        font_weight: "font-weight",
        format: "format",
        from: "from",
        fr: "fr",
        fx: "fx",
        fy: "fy",
        g1: "g1",
        g2: "g2",
        glyph_name: "glyph-name",
        glyph_orientation_horizontal: "glyph-orientation-horizontal",
        glyph_orientation_vertical: "glyph-orientation-vertical",
        glyphRef: "glyphRef",
        gradientTransform: "gradientTransform",
        gradientUnits: "gradientUnits",
        hanging: "hanging",
        height: "height",
        href: "href",
        hreflang: "hreflang",
        horiz_adv_x: "horiz-adv-x",
        horiz_origin_x: "horiz-origin-x",
        id: "id",
        ideographic: "ideographic",
        image_rendering: "image-rendering",
        _in: "_in",
        in2: "in2",
        intercept: "intercept",
        k: "k",
        k1: "k1",
        k2: "k2",
        k3: "k3",
        k4: "k4",
        kernelMatrix: "kernelMatrix",
        kernelUnitLength: "kernelUnitLength",
        kerning: "kerning",
        keyPoints: "keyPoints",
        keySplines: "keySplines",
        keyTimes: "keyTimes",
        lang: "lang",
        lengthAdjust: "lengthAdjust",
        letter_spacing: "letter-spacing",
        lighting_color: "lighting-color",
        limitingConeAngle: "limitingConeAngle",
        local: "local",
        marker_end: "marker-end",
        marker_mid: "marker-mid",
        marker_start: "marker_start",
        markerHeight: "markerHeight",
        markerUnits: "markerUnits",
        markerWidth: "markerWidth",
        mask: "mask",
        maskContentUnits: "maskContentUnits",
        maskUnits: "maskUnits",
        mathematical: "mathematical",
        max: "max",
        media: "media",
        method: "method",
        min: "min",
        mode: "mode",
        name: "name",
        numOctaves: "numOctaves",
        offset: "offset",
        opacity: "opacity",
        operator: "operator",
        order: "order",
        orient: "orient",
        orientation: "orientation",
        origin: "origin",
        overflow: "overflow",
        overline_position: "overline-position",
        overline_thickness: "overline-thickness",
        panose_1: "panose-1",
        paint_order: "paint-order",
        path: "path",
        pathLength: "pathLength",
        patternContentUnits: "patternContentUnits",
        patternTransform: "patternTransform",
        patternUnits: "patternUnits",
        ping: "ping",
        pointer_events: "pointer-events",
        points: "points",
        pointsAtX: "pointsAtX",
        pointsAtY: "pointsAtY",
        pointsAtZ: "pointsAtZ",
        preserveAlpha: "preserveAlpha",
        preserveAspectRatio: "preserveAspectRatio",
        primitiveUnits: "primitiveUnits",
        r: "r",
        radius: "radius",
        referrerPolicy: "referrerPolicy",
        refX: "refX",
        refY: "refY",
        rel: "rel",
        rendering_intent: "rendering-intent",
        repeatCount: "repeatCount",
        repeatDur: "repeatDur",
        requiredExtensions: "requiredExtensions",
        requiredFeatures: "requiredFeatures",
        restart: "restart",
        result: "result",
        role: "role",
        rotate: "rotate",
        rx: "rx",
        ry: "ry",
        scale: "scale",
        seed: "seed",
        shape_rendering: "shape-rendering",
        slope: "slope",
        spacing: "spacing",
        specularConstant: "specularConstant",
        specularExponent: "specularExponent",
        speed: "speed",
        spreadMethod: "spreadMethod",
        startOffset: "startOffset",
        stdDeviation: "stdDeviation",
        stemh: "stemh",
        stemv: "stemv",
        stitchTiles: "stitchTiles",
        stop_color: "stop_color",
        stop_opacity: "stop_opacity",
        strikethrough_position: "strikethrough-position",
        strikethrough_thickness: "strikethrough-thickness",
        string: "string",
        stroke: "stroke",
        stroke_dasharray: "stroke-dasharray",
        stroke_dashoffset: "stroke-dashoffset",
        stroke_linecap: "stroke-linecap",
        stroke_linejoin: "stroke-linejoin",
        stroke_miterlimit: "stroke-miterlimit",
        stroke_opacity: "stroke-opacity",
        stroke_width: "stroke-width",
        style: "style",
        surfaceScale: "surfaceScale",
        systemLanguage: "systemLanguage",
        tabindex: "tabindex",
        tableValues: "tableValues",
        target: "target",
        targetX: "targetX",
        targetY: "targetY",
        text_anchor: "text-anchor",
        text_decoration: "text-decoration",
        text_rendering: "text-rendering",
        textLength: "textLength",
        to: "to",
        transform: "transform",
        transform_origin: "transform-origin",
        r#type: "_type",
        u1: "u1",
        u2: "u2",
        underline_position: "underline-position",
        underline_thickness: "underline-thickness",
        unicode: "unicode",
        unicode_bidi: "unicode-bidi",
        unicode_range: "unicode-range",
        units_per_em: "units-per-em",
        v_alphabetic: "v-alphabetic",
        v_hanging: "v-hanging",
        v_ideographic: "v-ideographic",
        v_mathematical: "v-mathematical",
        values: "values",
        vector_effect: "vector-effect",
        version: "version",
        vert_adv_y: "vert-adv-y",
        vert_origin_x: "vert-origin-x",
        vert_origin_y: "vert-origin-y",
        view_box: "viewBox",
        view_target: "viewTarget",
        visibility: "visibility",
        width: "width",
        widths: "widths",
        word_spacing: "word-spacing",
        writing_mode: "writing-mode",
        x: "x",
        x_height: "x-height",
        x1: "x1",
        x2: "x2",
        xmlns: "xmlns",
        x_channel_selector: "xChannelSelector",
        y: "y",
        y1: "y1",
        y2: "y2",
        y_channel_selector: "yChannelSelector",
        z: "z",
        zoomAndPan: "zoomAndPan",
    }
}
