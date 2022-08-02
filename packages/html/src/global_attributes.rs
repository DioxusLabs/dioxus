use dioxus_core::*;

macro_rules! no_namespace_trait_methods {
    (
        $(
            $(#[$attr:meta])*
            $name:ident;
        )*
    ) => {
        $(
            #[allow(non_upper_case_globals)]
            const $name: AttributeDiscription = AttributeDiscription{
                name: stringify!($name),
                namespace: None,
                volatile: false
            };
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
            #[allow(non_upper_case_globals)]
            const $name: AttributeDiscription = AttributeDiscription{
                name: $lit,
                namespace: Some("style"),
                volatile: false
            };
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
            #[allow(non_upper_case_globals)]
            const $name: AttributeDiscription = AttributeDiscription{
                name: $lit,
                namespace: None,
                volatile: false
            };
        )*
    };
}

pub trait GlobalAttributes {
    /// Prevent the default action for this element.
    ///
    /// For more information, see the MDN docs:
    /// <https://developer.mozilla.org/en-US/docs/Web/API/Event/preventDefault>
    #[allow(non_upper_case_globals)]
    const prevent_default: AttributeDiscription = AttributeDiscription {
        name: "dioxus-prevent-default",
        namespace: None,
        volatile: false,
    };

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
        align_content: "align-content",
        align_items: "align-items",
        align_self: "align-self",
        alignment_adjust: "alignment-adjust",
        alignment_baseline: "alignment-baseline",
        all: "all",
        alt: "alt",
        animation: "animation",
        animation_delay: "animation-delay",
        animation_direction: "animation-direction",
        animation_duration: "animation-duration",
        animation_fill_mode: "animation-fill-mode",
        animation_iteration_count: "animation-iteration-count",
        animation_name: "animation-name",
        animation_play_state: "animation-play-state",
        animation_timing_function: "animation-timing-function",
        azimuth: "azimuth",
        backface_visibility: "backface-visibility",
        background: "background",
        background_attachment: "background-attachment",
        background_clip: "background-clip",
        background_color: "background-color",
        background_image: "background-image",
        background_origin: "background-origin",
        background_position: "background-position",
        background_repeat: "background-repeat",
        background_size: "background-size",
        background_blend_mode: "background-blend-mode",
        baseline_shift: "baseline-shift",
        bleed: "bleed",
        bookmark_label: "bookmark-label",
        bookmark_level: "bookmark-level",
        bookmark_state: "bookmark-state",
        border: "border",
        border_color: "border-color",
        border_style: "border-style",
        border_width: "border-width",
        border_bottom: "border-bottom",
        border_bottom_color: "border-bottom-color",
        border_bottom_style: "border-bottom-style",
        border_bottom_width: "border-bottom-width",
        border_left: "border-left",
        border_left_color: "border-left-color",
        border_left_style: "border-left-style",
        border_left_width: "border-left-width",
        border_right: "border-right",
        border_right_color: "border-right-color",
        border_right_style: "border-right-style",
        border_right_width: "border-right-width",
        border_top: "border-top",
        border_top_color: "border-top-color",
        border_top_style: "border-top-style",
        border_top_width: "border-top-width",
        border_collapse: "border-collapse",
        border_image: "border-image",
        border_image_outset: "border-image-outset",
        border_image_repeat: "border-image-repeat",
        border_image_slice: "border-image-slice",
        border_image_source: "border-image-source",
        border_image_width: "border-image-width",
        border_radius: "border-radius",
        border_bottom_left_radius: "border-bottom-left-radius",
        border_bottom_right_radius: "border-bottom-right-radius",
        border_top_left_radius: "border-top-left-radius",
        border_top_right_radius: "border-top-right-radius",
        border_spacing: "border-spacing",
        bottom: "bottom",
        box_decoration_break: "box-decoration-break",
        box_shadow: "box-shadow",
        box_sizing: "box-sizing",
        box_snap: "box-snap",
        break_after: "break-after",
        break_before: "break-before",
        break_inside: "break-inside",
        buffered_rendering: "buffered-rendering",
        caption_side: "caption-side",
        clear: "clear",
        clear_side: "clear-side",
        clip: "clip",
        clip_path: "clip-path",
        clip_rule: "clip-rule",
        color: "color",
        color_adjust: "color-adjust",
        color_correction: "color-correction",
        color_interpolation: "color-interpolation",
        color_interpolation_filters: "color-interpolation-filters",
        color_profile: "color-profile",
        color_rendering: "color-rendering",
        column_fill: "column-fill",
        column_gap: "column-gap",
        column_rule: "column-rule",
        column_rule_color: "column-rule-color",
        column_rule_style: "column-rule-style",
        column_rule_width: "column-rule-width",
        column_span: "column-span",
        columns: "columns",
        column_count: "column-count",
        column_width: "column-width",
        contain: "contain",
        content: "content",
        counter_increment: "counter-increment",
        counter_reset: "counter-reset",
        counter_set: "counter-set",
        cue: "cue",
        cue_after: "cue-after",
        cue_before: "cue-before",
        cursor: "cursor",
        direction: "direction",
        display: "display",
        display_inside: "display-inside",
        display_outside: "display-outside",
        display_extras: "display-extras",
        display_box: "display-box",
        dominant_baseline: "dominant-baseline",
        elevation: "elevation",
        empty_cells: "empty-cells",
        enable_background: "enable-background",
        fill: "fill",
        fill_opacity: "fill-opacity",
        fill_rule: "fill-rule",
        filter: "filter",
        float: "float",
        float_defer_column: "float-defer-column",
        float_defer_page: "float-defer-page",
        float_offset: "float-offset",
        float_wrap: "float-wrap",
        flow_into: "flow-into",
        flow_from: "flow-from",
        flex: "flex",
        flex_basis: "flex-basis",
        flex_grow: "flex-grow",
        flex_shrink: "flex-shrink",
        flex_flow: "flex-flow",
        flex_direction: "flex-direction",
        flex_wrap: "flex-wrap",
        flood_color: "flood-color",
        flood_opacity: "flood-opacity",
        font: "font",
        font_family: "font-family",
        font_size: "font-size",
        font_stretch: "font-stretch",
        font_style: "font-style",
        font_weight: "font-weight",
        font_feature_settings: "font-feature-settings",
        font_kerning: "font-kerning",
        font_language_override: "font-language-override",
        font_size_adjust: "font-size-adjust",
        font_synthesis: "font-synthesis",
        font_variant: "font-variant",
        font_variant_alternates: "font-variant-alternates",
        font_variant_caps: "font-variant-caps",
        font_variant_east_asian: "font-variant-east-asian",
        font_variant_ligatures: "font-variant-ligatures",
        font_variant_numeric: "font-variant-numeric",
        font_variant_position: "font-variant-position",
        footnote_policy: "footnote-policy",
        glyph_orientation_horizontal: "glyph-orientation-horizontal",
        glyph_orientation_vertical: "glyph-orientation-vertical",
        grid: "grid",
        grid_auto_flow: "grid-auto-flow",
        grid_auto_columns: "grid-auto-columns",
        grid_auto_rows: "grid-auto-rows",
        grid_template: "grid-template",
        grid_template_areas: "grid-template-areas",
        grid_template_columns: "grid-template-columns",
        grid_template_rows: "grid-template-rows",
        grid_area: "grid-area",
        grid_column: "grid-column",
        grid_column_start: "grid-column-start",
        grid_column_end: "grid-column-end",
        grid_row: "grid-row",
        grid_row_start: "grid-row-start",
        grid_row_end: "grid-row-end",
        hanging_punctuation: "hanging-punctuation",
        height: "height",
        hyphenate_character: "hyphenate-character",
        hyphenate_limit_chars: "hyphenate-limit-chars",
        hyphenate_limit_last: "hyphenate-limit-last",
        hyphenate_limit_lines: "hyphenate-limit-lines",
        hyphenate_limit_zone: "hyphenate-limit-zone",
        hyphens: "hyphens",
        icon: "icon",
        image_orientation: "image-orientation",
        image_resolution: "image-resolution",
        image_rendering: "image-rendering",
        ime: "ime",
        ime_align: "ime-align",
        ime_mode: "ime-mode",
        ime_offset: "ime-offset",
        ime_width: "ime-width",
        initial_letters: "initial-letters",
        inline_box_align: "inline-box-align",
        isolation: "isolation",
        justify_content: "justify-content",
        justify_items: "justify-items",
        justify_self: "justify-self",
        kerning: "kerning",
        left: "left",
        letter_spacing: "letter-spacing",
        lighting_color: "lighting-color",
        line_box_contain: "line-box-contain",
        line_break: "line-break",
        line_grid: "line-grid",
        line_height: "line-height",
        line_slack: "line-slack",
        line_snap: "line-snap",
        list_style: "list-style",
        list_style_image: "list-style-image",
        list_style_position: "list-style-position",
        list_style_type: "list-style-type",
        margin: "margin",
        margin_bottom: "margin-bottom",
        margin_left: "margin-left",
        margin_right: "margin-right",
        margin_top: "margin-top",
        marker: "marker",
        marker_end: "marker-end",
        marker_mid: "marker-mid",
        marker_pattern: "marker-pattern",
        marker_segment: "marker-segment",
        marker_start: "marker-start",
        marker_knockout_left: "marker-knockout-left",
        marker_knockout_right: "marker-knockout-right",
        marker_side: "marker-side",
        marks: "marks",
        marquee_direction: "marquee-direction",
        marquee_play_count: "marquee-play-count",
        marquee_speed: "marquee-speed",
        marquee_style: "marquee-style",
        mask: "mask",
        mask_image: "mask-image",
        mask_repeat: "mask-repeat",
        mask_position: "mask-position",
        mask_clip: "mask-clip",
        mask_origin: "mask-origin",
        mask_size: "mask-size",
        mask_box: "mask-box",
        mask_box_outset: "mask-box-outset",
        mask_box_repeat: "mask-box-repeat",
        mask_box_slice: "mask-box-slice",
        mask_box_source: "mask-box-source",
        mask_box_width: "mask-box-width",
        mask_type: "mask-type",
        max_height: "max-height",
        max_lines: "max-lines",
        max_width: "max-width",
        min_height: "min-height",
        min_width: "min-width",
        mix_blend_mode: "mix-blend-mode",
        nav_down: "nav-down",
        nav_index: "nav-index",
        nav_left: "nav-left",
        nav_right: "nav-right",
        nav_up: "nav-up",
        object_fit: "object-fit",
        object_position: "object-position",
        offset_after: "offset-after",
        offset_before: "offset-before",
        offset_end: "offset-end",
        offset_start: "offset-start",
        opacity: "opacity",
        order: "order",
        orphans: "orphans",
        outline: "outline",
        outline_color: "outline-color",
        outline_style: "outline-style",
        outline_width: "outline-width",
        outline_offset: "outline-offset",
        overflow: "overflow",
        overflow_x: "overflow-x",
        overflow_y: "overflow-y",
        overflow_style: "overflow-style",
        overflow_wrap: "overflow-wrap",
        padding: "padding",
        padding_bottom: "padding-bottom",
        padding_left: "padding-left",
        padding_right: "padding-right",
        padding_top: "padding-top",
        page: "page",
        page_break_after: "page-break-after",
        page_break_before: "page-break-before",
        page_break_inside: "page-break-inside",
        paint_order: "paint-order",
        pause: "pause",
        pause_after: "pause-after",
        pause_before: "pause-before",
        perspective: "perspective",
        perspective_origin: "perspective-origin",
        pitch: "pitch",
        pitch_range: "pitch-range",
        play_during: "play-during",
        pointer_events: "pointer-events",
        position: "position",
        quotes: "quotes",
        region_fragment: "region-fragment",
        resize: "resize",
        rest: "rest",
        rest_after: "rest-after",
        rest_before: "rest-before",
        richness: "richness",
        right: "right",
        ruby_align: "ruby-align",
        ruby_merge: "ruby-merge",
        ruby_position: "ruby-position",
        scroll_behavior: "scroll-behavior",
        scroll_snap_coordinate: "scroll-snap-coordinate",
        scroll_snap_destination: "scroll-snap-destination",
        scroll_snap_points_x: "scroll-snap-points-x",
        scroll_snap_points_y: "scroll-snap-points-y",
        scroll_snap_type: "scroll-snap-type",
        shape_image_threshold: "shape-image-threshold",
        shape_inside: "shape-inside",
        shape_margin: "shape-margin",
        shape_outside: "shape-outside",
        shape_padding: "shape-padding",
        shape_rendering: "shape-rendering",
        size: "size",
        speak: "speak",
        speak_as: "speak-as",
        speak_header: "speak-header",
        speak_numeral: "speak-numeral",
        speak_punctuation: "speak-punctuation",
        speech_rate: "speech-rate",
        stop_color: "stop-color",
        stop_opacity: "stop-opacity",
        stress: "stress",
        string_set: "string-set",
        stroke: "stroke",
        stroke_dasharray: "stroke-dasharray",
        stroke_dashoffset: "stroke-dashoffset",
        stroke_linecap: "stroke-linecap",
        stroke_linejoin: "stroke-linejoin",
        stroke_miterlimit: "stroke-miterlimit",
        stroke_opacity: "stroke-opacity",
        stroke_width: "stroke-width",
        tab_size: "tab-size",
        table_layout: "table-layout",
        text_align: "text-align",
        text_align_all: "text-align-all",
        text_align_last: "text-align-last",
        text_anchor: "text-anchor",
        text_combine_upright: "text-combine-upright",
        text_decoration: "text-decoration",
        text_decoration_color: "text-decoration-color",
        text_decoration_line: "text-decoration-line",
        text_decoration_style: "text-decoration-style",
        text_decoration_skip: "text-decoration-skip",
        text_emphasis: "text-emphasis",
        text_emphasis_color: "text-emphasis-color",
        text_emphasis_style: "text-emphasis-style",
        text_emphasis_position: "text-emphasis-position",
        text_emphasis_skip: "text-emphasis-skip",
        text_height: "text-height",
        text_indent: "text-indent",
        text_justify: "text-justify",
        text_orientation: "text-orientation",
        text_overflow: "text-overflow",
        text_rendering: "text-rendering",
        text_shadow: "text-shadow",
        text_size_adjust: "text-size-adjust",
        text_space_collapse: "text-space-collapse",
        text_spacing: "text-spacing",
        text_transform: "text-transform",
        text_underline_position: "text-underline-position",
        text_wrap: "text-wrap",
        top: "top",
        touch_action: "touch-action",
        transform: "transform",
        transform_box: "transform-box",
        transform_origin: "transform-origin",
        transform_style: "transform-style",
        transition: "transition",
        transition_delay: "transition-delay",
        transition_duration: "transition-duration",
        transition_property: "transition-property",
        unicode_bidi: "unicode-bidi",
        vector_effect: "vector-effect",
        vertical_align: "vertical-align",
        visibility: "visibility",
        voice_balance: "voice-balance",
        voice_duration: "voice-duration",
        voice_family: "voice-family",
        voice_pitch: "voice-pitch",
        voice_range: "voice-range",
        voice_rate: "voice-rate",
        voice_stress: "voice-stress",
        voice_volumn: "voice-volumn",
        volume: "volume",
        white_space: "white-space",
        widows: "widows",
        width: "width",
        will_change: "will-change",
        word_break: "word-break",
        word_spacing: "word-spacing",
        word_wrap: "word-wrap",
        wrap_flow: "wrap-flow",
        wrap_through: "wrap-through",
        writing_mode: "writing-mode",
        gap: "gap",
        list_styler_type: "list-style-type",
        row_gap: "row-gap",
        transition_timing_function: "transition-timing-function",
        user_select: "user-select",
        webkit_user_select: "-webkit-user-select",
        z_index : "z-index",
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
    /// Prevent the default action for this element.
    ///
    /// For more information, see the MDN docs:
    /// <https://developer.mozilla.org/en-US/docs/Web/API/Event/preventDefault>
    #[allow(non_upper_case_globals)]
    const prevent_default: AttributeDiscription = AttributeDiscription {
        name: "dioxus-prevent-default",
        namespace: None,
        volatile: false,
    };
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
