use bon::Builder;
use dioxus::dioxus_core::FunctionComponent;
use dioxus::prelude::*;
use dioxus_builder::document::{doc_stylesheet, doc_title};
use dioxus_builder::*;

const TAILWIND_CSS: Asset = asset!("/assets/tailwind.css");

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    // Using the new doc_title and doc_stylesheet helpers
    fragment()
        .child(doc_title("Dioxus Builder"))
        .child(doc_stylesheet(TAILWIND_CSS))
        .child(body_section())
        .build()
}

// =============================================================================
// Card Component - Demonstrates bon::Builder integration with Props
// =============================================================================

/// A simple card component that demonstrates using bon::Builder with Props.
///
/// When you add `#[props(bon)]`, the Props derive delegates builder generation
/// to bon, giving you a clean builder API without manual implementation.
///
/// Note: `#[props(extends = ...)]` is NOT supported with bon. Use standard
/// Props derive for components that need attribute spreading.
// Note: bon::Builder MUST come before Props in the derive list!
// This ensures bon generates the builder before Props tries to reference it.
#[derive(Clone, PartialEq, Builder, Props)]
#[props(bon)]
pub struct CardProps {
    /// The title of the card
    #[builder(into)]
    pub title: String,

    /// Optional subtitle - bon auto-defaults Option<T> to None
    #[builder(into)]
    pub subtitle: Option<String>,

    /// Whether the card has a border
    #[builder(default)]
    pub bordered: bool,

    /// The variant/style of the card
    #[builder(default)]
    pub variant: CardVariant,

    /// The children of the card
    pub children: Element,
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum CardVariant {
    #[default]
    Default,
    Primary,
}

/// A simple Card component using bon-generated props builder.
///
/// Usage with FunctionComponent trait:
/// ```ignore
/// Card.new()
///     .title("My Card")
///     .subtitle("A subtitle")
///     .bordered(true)
///     .variant(CardVariant::Primary)
///     .children(rsx! { "Content" })
///     .build()
/// ```
#[allow(non_snake_case)]
pub fn Card(props: CardProps) -> Element {
    let CardProps {
        title,
        subtitle,
        bordered,
        variant,
        children,
    } = props;

    let bg_class = match variant {
        CardVariant::Default => "bg-white",
        CardVariant::Primary => "bg-blue-50",
    };

    let border_class = match variant {
        CardVariant::Default => "border-gray-200",
        CardVariant::Primary => "border-blue-200",
    };

    div()
        .class_list(["rounded-lg", "shadow-sm", "overflow-hidden", bg_class])
        .class_if(bordered, border_class)
        .class_if(bordered, "border")
        .child(
            div()
                .class("p-4")
                .child(
                    h3().class("text-lg font-semibold text-gray-800")
                        .text(&title),
                )
                .child_option(subtitle.map(|s| p().class("text-sm text-gray-600 mt-1").text(s))),
        )
        .child(div().class("p-4 pt-0").child(children))
        .build()
}

fn body_section() -> Element {
    div()
        .class_list([
            "flex",
            "flex-col",
            "items-center",
            "justify-center",
            "min-h-screen",
            "bg-gray-100",
            "p-8",
            "space-y-6",
        ])
        .child(header_section())
        .child(bon_builder_section())
        .child(footer_section())
        .build()
}

fn header_section() -> Element {
    div()
        .class_list(["container", "mx-auto", "p-4", "text-center", "space-y-2"])
        .child(
            h1().class("text-4xl font-bold text-blue-600")
                .text("Dioxus Builder"),
        )
        .child(
            p().class("text-lg text-gray-700")
                .text("A type-safe, fluent API for building UI without rsx!."),
        )
        .build()
}

fn footer_section() -> Element {
    footer()
        .class("mt-8 text-gray-400 text-sm")
        .text("Built with dioxus-builder and bon")
        .build()
}

// =============================================================================
// Bon Builder Section - Demonstrates bon::Builder integration
// =============================================================================

/// Demonstrates the bon::Builder integration with Props derive.
///
/// Key features shown:
/// 1. `#[derive(bon::Builder, Props)]` with `#[props(bon)]` for automatic builder
/// 2. `FunctionComponent` trait for `Component.new()` syntax
/// 3. bon attributes like `#[builder(into)]`, `#[builder(default)]`
fn bon_builder_section() -> Element {
    div()
        .class_list([
            "mt-4",
            "w-full",
            "max-w-md",
            "bg-white",
            "shadow-xl",
            "rounded-lg",
            "p-4",
            "space-y-4",
        ])
        .child(
            h2().class("font-semibold text-lg text-purple-800")
                .text("bon::Builder Components"),
        )
        .child(
            p().class("text-sm text-gray-600")
                .text("Use #[derive(bon::Builder, Props)] with #[props(bon)] for a typed builder."),
        )
        // Example 1: Using CardProps::builder() directly
        .child(
            div()
                .class("space-y-2")
                .child(
                    p().class("text-xs font-medium text-gray-500")
                        .text("Method 1: CardProps::builder()"),
                )
                .child(Card(
                    CardProps::builder()
                        .title("Default Card")
                        .children(
                            p().class("text-gray-600")
                                .text("Built from props::builder()")
                                .build(),
                        )
                        .build(),
                )),
        )
        // Example 2: Using FunctionComponent trait - Card.new()
        .child(
            div()
                .class("space-y-2")
                .child(
                    p().class("text-xs font-medium text-gray-500")
                        .text("Method 2: Card.new() via FunctionComponent trait"),
                )
                .child(Card(
                    Card.new()
                        .title("Primary Card")
                        .subtitle("Using Card.new() syntax")
                        .variant(CardVariant::Primary)
                        .bordered(true)
                        .children(
                            p().class("text-gray-600")
                                .text("This uses the FunctionComponent trait!")
                                .build(),
                        )
                        .build(),
                )),
        )
        .build()
}
