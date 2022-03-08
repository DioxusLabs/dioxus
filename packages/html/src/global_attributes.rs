use dioxus_core::IntoVNode;

pub use crate::builder::IntoAttributeValue;
use crate::events::*;
use crate::{builder::ElementBuilder, element_builder::AnyBuilder};

// todo: fix up the other macros to generate their expansions
#[allow(unused)]
macro_rules! no_namespace_trait_methods {
    (
        $(
            $(#[$attr:meta])*
            $name:ident;
        )*
    ) => {
        $(
            $(#[$attr])*
            fn $name(&mut self, val: impl IntoAttributeValue<'a>) -> &mut Self {
                self.builder().attr(stringify!($name), val);
                self
            }
        )*
    };
}

#[allow(unused)]
macro_rules! events {
    ( $(
        $( #[$attr:meta] )*
        $data:ident: [
            $(
                $( #[$method_attr:meta] )*
                $name:ident
            )*
        ];
    )* ) => {
        $(
            $(
                $(#[$method_attr])*
                fn $name(&mut self, cb: impl FnMut(&$data) + 'a) -> &mut Self {
                    self.builder().push_listener(stringify!($name), cb); self
                }
            )*
        )*
    };
}

#[rustfmt::skip]
pub trait HtmlElement<'a> {
    fn builder(&mut self) -> &mut ElementBuilder<'a>;

    fn children<const LEN: usize>(&mut self, children: [&'a mut dyn AnyBuilder; LEN]) -> &mut Self {
        todo!()
    }

    fn children_from_iter(&mut self, children: impl IntoIterator<Item = impl IntoVNode<'a>>) -> &mut Self {
        todo!()
    }

    fn text(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self }

    fn accesskey(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("accesskey", f); self }
    fn contenteditable(&mut self, f: bool) -> &mut Self { self.builder().bool_attr("contenteditable", f); self }
    fn data(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("data", f); self }

    /// Add a single classname
    fn classname(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("classname", f); self }

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
    fn class(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("class", f); self }

    /// Set the direction of the text
    fn dir(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("dir", f); self }

    /// Set whether or not the element is hidden
    fn hidden(&mut self, f: bool) -> &mut Self { self.builder().bool_attr("hidden", f); self }

    /// Set whether or not the element is draggable
    fn draggable(&mut self, f: bool) -> &mut Self { self.builder().bool_attr("draggable", f); self }

    /// Set the ID of the element
    fn id(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("id", f); self }


    fn spellcheck(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("spellcheck", f); self }


    fn style(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("style", f); self }


    fn tabindex(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("tabindex", f); self }


    fn title(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("title", f); self }


    fn translate(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("translate", f); self }


    fn role(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("role", f); self }


    fn prevent_default(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("prevent_default", f); self }

    /// dangerous_inner_html is Dioxus's replacement for using innerHTML in the browser DOM. In general, setting
    /// HTML from code is risky because it’s easy to inadvertently expose your users to a cross-site scripting (XSS)
    /// attack. So, you can set HTML directly from Dioxus, but you have to type out dangerous_inner_html to remind
    /// yourself that it’s dangerous
    fn dangerous_inner_html(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("dangerous_inner_html", f); self }

    /// Specifies the alignment of flexible container's items within the flex container.
    fn align_content(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("align-content", f); self }

    /// Specifies the default alignment for items within the flex container.
    fn align_items(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("align-items", f); self }

    /// Specifies the alignment for selected items within the flex container.
    fn align_self(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("align-self", f); self }

    /// Specifies the keyframe_based animations.
    fn animation(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("animation", f); self }

    /// Specifies when the animation will start.
    fn animation_delay(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("animation-delay", f); self }

    /// Specifies whether the animation should play in reverse on alternate cycles or not.
    fn animation_direction(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("animation-direction", f); self }

    /// Specifies the number of seconds or milliseconds an animation should take to complete one cycle
    fn animation_duration(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("animation-duration", f); self }

    /// Specifies how a CSS animation should apply styles to its target before and after it is executing
    fn animation_fill_mode(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("animation-fill-mode", f); self }

    /// Specifies the number of times an animation cycle should be played before stopping.
    fn animation_iteration_count(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("animation-iteration-count", f); self }

    /// Specifies the name of @keyframes defined animations that should be applied to the selected element
    fn animation_name(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("animation-name", f); self }

    /// Specifies whether the animation is running or paused.
    fn animation_play_state(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("animation-play-state", f); self }

    /// Specifies how a CSS animation should progress over the duration of each cycle.
    fn animation_timing_function(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("animation-timing-function", f); self }

    /// Specifies whether or not the "back" side of a transformed element is visible when facing the user.
    fn backface_visibility(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("backface-visibility", f); self }

    /// Defines a variety of background properties within one declaration.
    fn background(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("background", f); self }

    /// Specify whether the background image is fixed in the viewport or scrolls.
    fn background_attachment(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("background-attachment", f); self }

    /// Specifies the painting area of the background.
    fn background_clip(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("background-clip", f); self }

    /// Defines an element's background color.
    fn background_color(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("background-color", f); self }

    /// Defines an element's background image.
    fn background_image(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("background-image", f); self }

    /// Specifies the positioning area of the background images.
    fn background_origin(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("background-origin", f); self }

    /// Defines the origin of a background image.
    fn background_position(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("background-position", f); self }

    /// Specify whether/how the background image is tiled.
    fn background_repeat(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("background-repeat", f); self }

    /// Specifies the size of the background images.
    fn background_size(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("background-size", f); self }

    /// Sets the width, style, and color for all four sides of an element's border.
    fn border(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border", f); self }

    /// Sets the width, style, and color of the bottom border of an element.
    fn border_bottom(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-bottom", f); self }

    /// Sets the color of the bottom border of an element.
    fn border_bottom_color(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-bottom-color", f); self }

    /// Defines the shape of the bottom_left border corner of an element.
    fn border_bottom_left_radius(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-bottom-left-radius", f); self }

    /// Defines the shape of the bottom_right border corner of an element.
    fn border_bottom_right_radius(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-bottom-right-radius", f); self }

    /// Sets the style of the bottom border of an element.
    fn border_bottom_style(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-bottom-style", f); self }

    /// Sets the width of the bottom border of an element.
    fn border_bottom_width(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-bottom-width", f); self }

    /// Specifies whether table cell borders are connected or separated.
    fn border_collapse(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-collapse", f); self }

    /// Sets the color of the border on all the four sides of an element.
    fn border_color(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-color", f); self }

    /// Specifies how an image is to be used in place of the border styles.
    fn border_image(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-image", f); self }

    /// Specifies the amount by which the border image area extends beyond the border box.
    fn border_image_outset(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-image-outset", f); self }

    /// Specifies whether the image_border should be repeated, rounded or stretched.
    fn border_image_repeat(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-image-repeat", f); self }

    /// Specifies the inward offsets of the image_border.
    fn border_image_slice(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-image-slice", f); self }

    /// Specifies the location of the image to be used as a border.
    fn border_image_source(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-image-source", f); self }

    /// Specifies the width of the image_border.
    fn border_image_width(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-image-width", f); self }

    /// Sets the width, style, and color of the left border of an element.
    fn border_left(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-left", f); self }

    /// Sets the color of the left border of an element.
    fn border_left_color(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-left-color", f); self }

    /// Sets the style of the left border of an element.
    fn border_left_style(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-left-style", f); self }

    /// Sets the width of the left border of an element.
    fn border_left_width(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-left-width", f); self }

    /// Defines the shape of the border corners of an element.
    fn border_radius(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-radius", f); self }

    /// Sets the width, style, and color of the right border of an element.
    fn border_right(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-right", f); self }

    /// Sets the color of the right border of an element.
    fn border_right_color(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-right-color", f); self }

    /// Sets the style of the right border of an element.
    fn border_right_style(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-right-style", f); self }

    /// Sets the width of the right border of an element.
    fn border_right_width(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-right-width", f); self }

    /// Sets the spacing between the borders of adjacent table cells.
    fn border_spacing(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-spacing", f); self }

    /// Sets the style of the border on all the four sides of an element.
    fn border_style(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-style", f); self }

    /// Sets the width, style, and color of the top border of an element.
    fn border_top(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-top", f); self }

    /// Sets the color of the top border of an element.
    fn border_top_color(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-top-color", f); self }

    /// Defines the shape of the top_left border corner of an element.
    fn border_top_left_radius(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-top-left-radius", f); self }

    /// Defines the shape of the top_right border corner of an element.
    fn border_top_right_radius(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-top-right-radius", f); self }

    /// Sets the style of the top border of an element.
    fn border_top_style(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-top-style", f); self }

    /// Sets the width of the top border of an element.
    fn border_top_width(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-top-width", f); self }

    /// Sets the width of the border on all the four sides of an element.
    fn border_width(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("border-width", f); self }

    /// Specify the location of the bottom edge of the positioned element.
    fn bottom(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("bottom", f); self }

    /// Applies one or more drop_shadows to the element's box.
    fn box_shadow(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("box-shadow", f); self }

    /// Alter the default CSS box model.
    fn box_sizing(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("box-sizing", f); self }

    /// Specify the position of table's caption.
    fn caption_side(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("caption-side", f); self }

    /// Specifies the placement of an element in relation to floating elements.
    fn clear(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("clear", f); self }

    /// Defines the clipping region.
    fn clip(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("clip", f); self }

    /// Specify the color of the text of an element.
    fn color(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("color", f); self }

    /// Specifies the number of columns in a multi_column element.
    fn column_count(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("column-count", f); self }

    /// Specifies how columns will be filled.
    fn column_fill(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("column-fill", f); self }

    /// Specifies the gap between the columns in a multi_column element.
    fn column_gap(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("column-gap", f); self }

    /// Specifies a straight line, or "rule", to be drawn between each column in a multi_column element.
    fn column_rule(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("column-rule", f); self }

    /// Specifies the color of the rules drawn between columns in a multi_column layout.
    fn column_rule_color(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("column-rule-color", f); self }

    /// Specifies the style of the rule drawn between the columns in a multi_column layout.
    fn column_rule_style(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("column-rule-style", f); self }

    /// Specifies the width of the rule drawn between the columns in a multi_column layout.
    fn column_rule_width(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("column-rule-width", f); self }

    /// Specifies how many columns an element spans across in a multi_column layout.
    fn column_span(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("column-span", f); self }

    /// Specifies the optimal width of the columns in a multi_column element.
    fn column_width(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("column-width", f); self }

    /// A shorthand property for setting column_width and column_count properties.
    fn columns(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("columns", f); self }

    /// Inserts generated content.
    fn content(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("content", f); self }

    /// Increments one or more counter values.
    fn counter_increment(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("counter-increment", f); self }

    /// Creates or resets one or more counters.
    fn counter_reset(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("counter-reset", f); self }

    /// Specify the type of cursor.
    fn cursor(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("cursor", f); self }

    /// Define the text direction/writing direction.
    fn direction(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("direction", f); self }

    /// Specifies how an element is displayed onscreen.
    fn display(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("display", f); self }

    /// Show or hide borders and backgrounds of empty table cells.
    fn empty_cells(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("empty-cells", f); self }

    /// Specifies the components of a flexible length.
    fn flex(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("flex", f); self }

    /// Specifies the initial main size of the flex item.
    fn flex_basis(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("flex-basis", f); self }

    /// Specifies the direction of the flexible items.
    fn flex_direction(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("flex-direction", f); self }

    /// A shorthand property for the flex_direction and the flex_wrap properties.
    fn flex_flow(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("flex-flow", f); self }

    /// Specifies how the flex item will grow relative to the other items inside the flex container.
    fn flex_grow(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("flex-grow", f); self }

    /// Specifies how the flex item will shrink relative to the other items inside the flex container
    fn flex_shrink(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("flex-shrink", f); self }

    /// Specifies whether the flexible items should wrap or not.
    fn flex_wrap(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("flex-wrap", f); self }

    /// Specifies whether or not a box should float.
    fn float(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("float", f); self }

    /// Defines a variety of font properties within one declaration.
    fn font(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("font", f); self }

    /// Defines a list of fonts for element.
    fn font_family(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("font-family", f); self }

    /// Defines the font size for the text.
    fn font_size(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("font-size", f); self }

    /// Preserves the readability of text when font fallback occurs.
    fn font_size_adjust(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("font-size-adjust", f); self }

    /// Selects a normal, condensed, or expanded face from a font.
    fn font_stretch(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("font-stretch", f); self }

    /// Defines the font style for the text.
    fn font_style(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("font-style", f); self }

    /// Specify the font variant.
    fn font_variant(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("font-variant", f); self }

    /// Specify the font weight of the text.
    fn font_weight(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("font-weight", f); self }

    /// Sets gaps (gutters) between rows and columns. Shorthand for row_gap and column_gap.
    fn gap(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("gap", f); self }

    /// Specify the height of an element.
    fn height(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("height", f); self }

    /// Specifies how flex items are aligned along the main axis of the flex container after any flexible lengths and auto margins have been resolved.
    fn justify_content(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("justify-content", f); self }

    /// Specify the location of the left edge of the positioned element.
    fn left(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("left", f); self }

    /// Sets the extra spacing between letters.
    fn letter_spacing(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("letter-spacing", f); self }

    /// Sets the height between lines of text.
    fn line_height(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("line-height", f); self }

    /// Defines the display style for a list and list elements.
    fn list_style(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("list-style", f); self }

    /// Specifies the image to be used as a list_item marker.
    fn list_style_image(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("list-style-image", f); self }

    /// Specifies the position of the list_item marker.
    fn list_style_position(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("list-style-position", f); self }

    /// Specifies the marker style for a list_item.
    fn list_styler_type(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("list-style-type", f); self }

    /// Sets the margin on all four sides of the element.
    fn margin(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("margin", f); self }

    /// Sets the bottom margin of the element.
    fn margin_bottom(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("margin-bottom", f); self }

    /// Sets the left margin of the element.
    fn margin_left(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("margin-left", f); self }

    /// Sets the right margin of the element.
    fn margin_right(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("margin-right", f); self }

    /// Sets the top margin of the element.
    fn margin_top(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("margin-top", f); self }

    /// Specify the maximum height of an element.
    fn max_height(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("max-height", f); self }

    /// Specify the maximum width of an element.
    fn max_width(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("max-width", f); self }

    /// Specify the minimum height of an element.
    fn min_height(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("min-height", f); self }

    /// Specify the minimum width of an element.
    fn min_width(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("min-width", f); self }

    /// Specifies the transparency of an element.
    fn opacity(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("opacity", f); self }

    /// Specifies the order in which a flex items are displayed and laid out within a flex container.
    fn order(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("order", f); self }

    /// Sets the width, style, and color for all four sides of an element's outline.
    fn outline(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("outline", f); self }

    /// Sets the color of the outline.
    fn outline_color(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("outline-color", f); self }

    /// Set the space between an outline and the border edge of an element.
    fn outline_offset(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("outline-offset", f); self }

    /// Sets a style for an outline.
    fn outline_style(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("outline-style", f); self }

    /// Sets the width of the outline.
    fn outline_width(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("outline-width", f); self }

    /// Specifies the treatment of content that overflows the element's box.
    fn overflow(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("overflow", f); self }

    /// Specifies the treatment of content that overflows the element's box horizontally.
    fn overflow_x(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("overflow-x", f); self }

    /// Specifies the treatment of content that overflows the element's box vertically.
    fn overflow_y(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("overflow-y", f); self }

    /// Sets the padding on all four sides of the element.
    fn padding(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("padding", f); self }

    /// Sets the padding to the bottom side of an element.
    fn padding_bottom(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("padding-bottom", f); self }

    /// Sets the padding to the left side of an element.
    fn padding_left(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("padding-left", f); self }

    /// Sets the padding to the right side of an element.
    fn padding_right(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("padding-right", f); self }

    /// Sets the padding to the top side of an element.
    fn padding_top(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("padding-top", f); self }

    /// Insert a page breaks after an element.
    fn page_break_after(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("page-break-after", f); self }

    /// Insert a page breaks before an element.
    fn page_break_before(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("page-break-before", f); self }

    /// Insert a page breaks inside an element.
    fn page_break_inside(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("page-break-inside", f); self }

    /// Defines the perspective from which all child elements of the object are viewed.
    fn perspective(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("perspective", f); self }

    /// Defines the origin (the vanishing point for the 3D space) for the perspective property.
    fn perspective_origin(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("perspective-origin", f); self }

    /// Specifies how an element is positioned.
    fn position(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("position", f); self }

    /// The pointer-events CSS property sets under what circumstances (if any) a particular graphic element can
    /// become the target of pointer events.
    ///
    /// MDN: [`pointer_events`](https://developer.mozilla.org/en-US/docs/Web/CSS/pointer-events)
    fn pointer_events(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("pointer-events", f); self }

    /// Specifies quotation marks for embedded quotations.
    fn quotes(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("quotes", f); self }

    /// Specifies whether or not an element is resizable by the user.
    fn resize(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("resize", f); self }

    /// Specify the location of the right edge of the positioned element.
    fn right(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("right", f); self }

    /// Specifies the gap between the rows in a multi_column element.
    fn row_gap(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("row-gap", f); self }

    /// Specifies the length of the tab character.
    fn tab_size(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("tab-size", f); self }

    /// Specifies a table layout algorithm.
    fn table_layout(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("table-layout", f); self }

    /// Sets the horizontal alignment of inline content.
    fn text_align(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("text-align", f); self }

    /// Specifies how the last line of a block or a line right before a forced line break is aligned when  is justify.",
    fn text_align_last(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("text-align-last", f); self }

    /// Specifies the decoration added to text.
    fn text_decoration(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("text-decoration", f); self }

    /// Specifies the color of the text_decoration_line.
    fn text_decoration_color(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("text-decoration-color", f); self }

    /// Specifies what kind of line decorations are added to the element.
    fn text_decoration_line(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("text-decoration-line", f); self }

    /// Specifies the style of the lines specified by the text_decoration_line property
    fn text_decoration_style(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("text-decoration-style", f); self }

    /// Indent the first line of text.
    fn text_indent(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("text-indent", f); self }

    /// Specifies the justification method to use when the text_align property is set to justify.
    fn text_justify(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("text-justify", f); self }

    /// Specifies how the text content will be displayed, when it overflows the block containers.
    fn text_overflow(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("text-overflow", f); self }

    /// Applies one or more shadows to the text content of an element.
    fn text_shadow(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("text-shadow", f); self }

    /// Transforms the case of the text.
    fn text_transform(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("text-transform", f); self }

    /// Specify the location of the top edge of the positioned element.
    fn top(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("top", f); self }

    /// Applies a 2D or 3D transformation to an element.
    fn transform(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("transform", f); self }

    /// Defines the origin of transformation for an element.
    fn transform_origin(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("transform-origin", f); self }

    /// Specifies how nested elements are rendered in 3D space.
    fn transform_style(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("transform-style", f); self }

    /// Defines the transition between two states of an element.
    fn transition(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("transition", f); self }

    /// Specifies when the transition effect will start.
    fn transition_delay(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("transition-delay", f); self }

    /// Specifies the number of seconds or milliseconds a transition effect should take to complete.
    fn transition_duration(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("transition-duration", f); self }

    /// Specifies the names of the CSS properties to which a transition effect should be applied.
    fn transition_property(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("transition-property", f); self }

    /// Specifies the speed curve of the transition effect.
    fn transition_timing_function(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("transition-timing-function", f); self }

    /// The user-select CSS property controls whether the user can select text.
    /// This doesn't have any effect on content loaded as part of a browser's user interface (its chrome), except in textboxes.
    fn user_select(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("user-select", f); self }
    fn webkit_user_select(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("-webkit-user-select", f); self }

    /// Sets the vertical positioning of an element relative to the current text baseline.
    fn vertical_align(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("vertical-align", f); self }

    /// Specifies whether or not an element is visible.
    fn visibility(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("visibility", f); self }

    /// Specifies how white space inside the element is handled.
    fn white_space(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("white-space", f); self }

    /// Specify the width of an element.
    fn width(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("width", f); self }

    /// Specifies how to break lines within words.
    fn word_break(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("word-break", f); self }

    /// Sets the spacing between words.
    fn word_spacing(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("word-spacing", f); self }

    /// Specifies whether to break words when the content overflows the boundaries of its container.
    fn word_wrap(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("word-wrap", f); self }

    /// Specifies a layering or stacking order for positioned elements.
    fn z_index(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self {  self.builder().style_attr("z-index", f); self }


    /*
    ARIA
    */

    fn aria_current(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-current", f); self }
    fn aria_details(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-details", f); self }
    fn aria_disabled(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-disabled", f); self }
    fn aria_hidden(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-hidden", f); self }
    fn aria_invalid(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-invalid", f); self }
    fn aria_keyshortcuts(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-keyshortcuts", f); self }
    fn aria_label(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-label", f); self }
    fn aria_roledescription(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-roledescription", f); self }

    // Widget Attributes
    fn aria_autocomplete(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-autocomplete", f); self }
    fn aria_checked(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-checked", f); self }
    fn aria_expanded(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-expanded", f); self }
    fn aria_haspopup(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-haspopup", f); self }
    fn aria_level(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-level", f); self }
    fn aria_modal(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-modal", f); self }
    fn aria_multiline(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-multiline", f); self }
    fn aria_multiselectable(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-multiselectable", f); self }
    fn aria_orientation(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-orientation", f); self }
    fn aria_placeholder(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-placeholder", f); self }
    fn aria_pressed(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-pressed", f); self }
    fn aria_readonly(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-readonly", f); self }
    fn aria_required(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-required", f); self }
    fn aria_selected(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-selected", f); self }
    fn aria_sort(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-sort", f); self }
    fn aria_valuemax(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-valuemax", f); self }
    fn aria_valuemin(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-valuemin", f); self }
    fn aria_valuenow(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-valuenow", f); self }
    fn aria_valuetext(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-valuetext", f); self }

    // Live Region Attributes
    fn aria_atomic(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-atomic", f); self }
    fn aria_busy(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-busy", f); self }
    fn aria_live(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-live", f); self }
    fn aria_relevant(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-relevant", f); self }
    fn aria_dropeffect(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-dropeffect", f); self }
    fn aria_grabbed(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-grabbed", f); self }

    // Relationship Attributes
    fn aria_activedescendant(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-activedescendant", f); self }
    fn aria_colcount(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-colcount", f); self }
    fn aria_colindex(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-colindex", f); self }
    fn aria_colspan(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-colspan", f); self }
    fn aria_controls(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-controls", f); self }
    fn aria_describedby(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-describedby", f); self }
    fn aria_errormessage(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-errormessage", f); self }
    fn aria_flowto(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-flowto", f); self }
    fn aria_labelledby(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-labelledby", f); self }
    fn aria_owns(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-owns", f); self }
    fn aria_posinset(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-posinset", f); self }
    fn aria_rowcount(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-rowcount", f); self }
    fn aria_rowindex(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-rowindex", f); self }
    fn aria_rowspan(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-rowspan", f); self }
    fn aria_setsize(&mut self, f: impl IntoAttributeValue<'a>) -> &mut Self { self.builder().attr("aria-setsize", f); self }


    // a little bit of a hack - we can just expand the macro and then paste the expansion here :)

    /// Called when \"copy\"
    fn oncopy(&mut self, cb: impl FnMut(&ClipboardEvent) + 'a) -> &mut Self { self.builder().push_listener("oncopy", cb); self }

    /// oncut
    fn oncut(&mut self, cb: impl FnMut(&ClipboardEvent) + 'a) -> &mut Self { self.builder().push_listener("oncut", cb); self }

    /// onpaste
    fn onpaste(&mut self, cb: impl FnMut(&ClipboardEvent) + 'a) -> &mut Self { self.builder().push_listener("onpaste", cb); self }

    /// oncompositionend
    fn oncompositionend(&mut self, cb: impl FnMut(&CompositionEvent) + 'a) -> &mut Self { self.builder().push_listener("oncompositionend", cb); self }

    /// oncompositionstart
    fn oncompositionstart(&mut self, cb: impl FnMut(&CompositionEvent) + 'a) -> &mut Self { self.builder().push_listener("oncompositionstart", cb); self }

    /// oncompositionupdate
    fn oncompositionupdate(&mut self, cb: impl FnMut(&CompositionEvent) + 'a) -> &mut Self { self.builder().push_listener("oncompositionupdate", cb); self }

    /// onkeydown
    fn onkeydown(&mut self, cb: impl FnMut(&KeyboardEvent) + 'a) -> &mut Self { self.builder().push_listener("onkeydown", cb); self }

    /// onkeypress
    fn onkeypress(&mut self, cb: impl FnMut(&KeyboardEvent) + 'a) -> &mut Self { self.builder().push_listener("onkeypress", cb); self }

    /// onkeyup
    fn onkeyup(&mut self, cb: impl FnMut(&KeyboardEvent) + 'a) -> &mut Self { self.builder().push_listener("onkeyup", cb); self }

    /// onfocus
    fn onfocus(&mut self, cb: impl FnMut(&FocusEvent) + 'a) -> &mut Self { self.builder().push_listener("onfocus", cb); self }

    fn onfocusout(&mut self, cb: impl FnMut(&FocusEvent) + 'a) -> &mut Self { self.builder().push_listener("onfocusout", cb); self }

    fn onfocusin(&mut self, cb: impl FnMut(&FocusEvent) + 'a) -> &mut Self { self.builder().push_listener("onfocusin", cb); self }

    /// onblur
    fn onblur(&mut self, cb: impl FnMut(&FocusEvent) + 'a) -> &mut Self { self.builder().push_listener("onblur", cb); self }

    /// onchange
    fn onchange(&mut self, cb: impl FnMut(&FormEvent) + 'a) -> &mut Self { self.builder().push_listener("onchange", cb); self }

    /// oninput handler
    fn oninput(&mut self, cb: impl FnMut(&FormEvent) + 'a) -> &mut Self { self.builder().push_listener("oninput", cb); self }

    /// oninvalid
    fn oninvalid(&mut self, cb: impl FnMut(&FormEvent) + 'a) -> &mut Self { self.builder().push_listener("oninvalid", cb); self }

    /// onreset
    fn onreset(&mut self, cb: impl FnMut(&FormEvent) + 'a) -> &mut Self { self.builder().push_listener("onreset", cb); self }

    /// onsubmit
    fn onsubmit(&mut self, cb: impl FnMut(&FormEvent) + 'a) -> &mut Self { self.builder().push_listener("onsubmit", cb); self }

    /// Execute a callback when a button is clicked.
    ///
    /// ## Description
    ///
    /// An element receives a click event when a pointing device button (such as a mouse\'s primary mouse button)
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
    /// rsx!( button { \"click me\", onclick: move |_| log::info!(\"Clicked!`\") } )
    /// ```
    ///
    /// ## Reference
    /// - <https://www.w3schools.com/tags/ev_onclick.asp>
    /// - <https://developer.mozilla.org/en-US/docs/Web/API/Element/click_event>
    fn onclick(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("onclick", cb); self }

    /// oncontextmenu
    fn oncontextmenu(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("oncontextmenu", cb); self }

    /// ondoubleclick
    fn ondoubleclick(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("ondoubleclick", cb); self }

    /// ondrag
    fn ondrag(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("ondrag", cb); self }

    /// ondragend
    fn ondragend(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("ondragend", cb); self }

    /// ondragenter
    fn ondragenter(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("ondragenter", cb); self }

    /// ondragexit
    fn ondragexit(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("ondragexit", cb); self }

    /// ondragleave
    fn ondragleave(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("ondragleave", cb); self }

    /// ondragover
    fn ondragover(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("ondragover", cb); self }

    /// ondragstart
    fn ondragstart(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("ondragstart", cb); self }

    /// ondrop
    fn ondrop(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("ondrop", cb); self }

    /// onmousedown
    fn onmousedown(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("onmousedown", cb); self }

    /// onmouseenter
    fn onmouseenter(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("onmouseenter", cb); self }

    /// onmouseleave
    fn onmouseleave(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("onmouseleave", cb); self }

    /// onmousemove
    fn onmousemove(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("onmousemove", cb); self }

    /// onmouseout
    fn onmouseout(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("onmouseout", cb); self }

    ///
    fn onscroll(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("onscroll", cb); self }

    /// onmouseover
    ///
    /// Triggered when the users\'s mouse hovers over an element.
    fn onmouseover(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("onmouseover", cb); self }

    /// onmouseup
    fn onmouseup(&mut self, cb: impl FnMut(&MouseEvent) + 'a) -> &mut Self { self.builder().push_listener("onmouseup", cb); self }

    /// pointerdown
    fn onpointerdown(&mut self, cb: impl FnMut(&PointerEvent) + 'a) -> &mut Self { self.builder().push_listener("onpointerdown", cb); self }

    /// pointermove
    fn onpointermove(&mut self, cb: impl FnMut(&PointerEvent) + 'a) -> &mut Self { self.builder().push_listener("onpointermove", cb); self }

    /// pointerup
    fn onpointerup(&mut self, cb: impl FnMut(&PointerEvent) + 'a) -> &mut Self { self.builder().push_listener("onpointerup", cb); self }

    /// pointercancel
    fn onpointercancel(&mut self, cb: impl FnMut(&PointerEvent) + 'a) -> &mut Self { self.builder().push_listener("onpointercancel", cb); self }

    /// gotpointercapture
    fn ongotpointercapture(&mut self, cb: impl FnMut(&PointerEvent) + 'a) -> &mut Self { self.builder().push_listener("ongotpointercapture", cb); self }

    /// lostpointercapture
    fn onlostpointercapture(&mut self, cb: impl FnMut(&PointerEvent) + 'a) -> &mut Self { self.builder().push_listener("onlostpointercapture", cb); self }

    /// pointerenter
    fn onpointerenter(&mut self, cb: impl FnMut(&PointerEvent) + 'a) -> &mut Self { self.builder().push_listener("onpointerenter", cb); self }

    /// pointerleave
    fn onpointerleave(&mut self, cb: impl FnMut(&PointerEvent) + 'a) -> &mut Self { self.builder().push_listener("onpointerleave", cb); self }

    /// pointerover
    fn onpointerover(&mut self, cb: impl FnMut(&PointerEvent) + 'a) -> &mut Self { self.builder().push_listener("onpointerover", cb); self }

    /// pointerout
    fn onpointerout(&mut self, cb: impl FnMut(&PointerEvent) + 'a) -> &mut Self { self.builder().push_listener("onpointerout", cb); self }

    /// onselect
    fn onselect(&mut self, cb: impl FnMut(&SelectionEvent) + 'a) -> &mut Self { self.builder().push_listener("onselect", cb); self }

    /// ontouchcancel
    fn ontouchcancel(&mut self, cb: impl FnMut(&TouchEvent) + 'a) -> &mut Self { self.builder().push_listener("ontouchcancel", cb); self }

    /// ontouchend
    fn ontouchend(&mut self, cb: impl FnMut(&TouchEvent) + 'a) -> &mut Self { self.builder().push_listener("ontouchend", cb); self }

    /// ontouchmove
    fn ontouchmove(&mut self, cb: impl FnMut(&TouchEvent) + 'a) -> &mut Self { self.builder().push_listener("ontouchmove", cb); self }

    /// ontouchstart
    fn ontouchstart(&mut self, cb: impl FnMut(&TouchEvent) + 'a) -> &mut Self { self.builder().push_listener("ontouchstart", cb); self }

    ///
    fn onwheel(&mut self, cb: impl FnMut(&WheelEvent) + 'a) -> &mut Self { self.builder().push_listener("onwheel", cb); self }

    ///abort
    fn onabort(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onabort", cb); self }

    ///canplay
    fn oncanplay(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("oncanplay", cb); self }

    ///canplaythrough
    fn oncanplaythrough(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("oncanplaythrough", cb); self }

    ///durationchange
    fn ondurationchange(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("ondurationchange", cb); self }

    ///emptied
    fn onemptied(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onemptied", cb); self }

    ///encrypted
    fn onencrypted(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onencrypted", cb); self }

    ///ended
    fn onended(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onended", cb); self }

    ///error
    fn onerror(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onerror", cb); self }

    ///loadeddata
    fn onloadeddata(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onloadeddata", cb); self }

    ///loadedmetadata
    fn onloadedmetadata(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onloadedmetadata", cb); self }

    ///loadstart
    fn onloadstart(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onloadstart", cb); self }

    ///pause
    fn onpause(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onpause", cb); self }

    ///play
    fn onplay(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onplay", cb); self }

    ///playing
    fn onplaying(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onplaying", cb); self }

    ///progress
    fn onprogress(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onprogress", cb); self }

    ///ratechange
    fn onratechange(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onratechange", cb); self }

    ///seeked
    fn onseeked(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onseeked", cb); self }

    ///seeking
    fn onseeking(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onseeking", cb); self }

    ///stalled
    fn onstalled(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onstalled", cb); self }

    ///suspend
    fn onsuspend(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onsuspend", cb); self }

    ///timeupdate
    fn ontimeupdate(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("ontimeupdate", cb); self }

    ///volumechange
    fn onvolumechange(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onvolumechange", cb); self }

    ///waiting
    fn onwaiting(&mut self, cb: impl FnMut(&MediaEvent) + 'a) -> &mut Self { self.builder().push_listener("onwaiting", cb); self }

    /// onanimationstart
    fn onanimationstart(&mut self, cb: impl FnMut(&AnimationEvent) + 'a) -> &mut Self { self.builder().push_listener("onanimationstart", cb); self }

    /// onanimationend
    fn onanimationend(&mut self, cb: impl FnMut(&AnimationEvent) + 'a) -> &mut Self { self.builder().push_listener("onanimationend", cb); self }

    /// onanimationiteration
    fn onanimationiteration(&mut self, cb: impl FnMut(&AnimationEvent) + 'a) -> &mut Self { self.builder().push_listener("onanimationiteration", cb); self }

    ///
    fn ontransitionend(&mut self, cb: impl FnMut(&TransitionEvent) + 'a) -> &mut Self { self.builder().push_listener("ontransitionend", cb); self }

    ///
    fn ontoggle(&mut self, cb: impl FnMut(&ToggleEvent) + 'a) -> &mut Self { self.builder().push_listener("ontoggle", cb); self }


}
