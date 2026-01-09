//! Tests for bon::Builder integration with Props derive.
//!
//! These tests verify that when `#[derive(bon::Builder, Props)]` is used together with
//! `#[props(bon)]`, the Props derive correctly delegates builder generation to bon while
//! still providing the Properties trait implementation.
//!
//! Note: The `#[props(bon)]` attribute is required to tell the Props derive to use bon's
//! builder instead of generating its own. This is because derive macros don't see other
//! derives in the same `#[derive(...)]` attribute.

use bon::Builder;
use dioxus::dioxus_core::{FunctionComponent, Properties};
use dioxus::prelude::*;

/// Test basic bon integration - Props derive should detect #[props(bon)] and generate
/// a simplified Properties impl that delegates to bon's builder.
#[test]
fn test_bon_basic_builder() {
    #[derive(Builder, Props, Clone, PartialEq)]
    #[props(bon)]
    struct TestProps {
        name: String,
        count: u32,
    }

    // Test that we can use the builder
    let props = TestProps::builder()
        .name("test".to_string())
        .count(42)
        .build();

    assert_eq!(props.name, "test");
    assert_eq!(props.count, 42);
}

/// Test that bon's default attribute works with Props.
#[test]
fn test_bon_with_defaults() {
    #[derive(Builder, Props, Clone, PartialEq)]
    #[props(bon)]
    struct TestProps {
        #[builder(default)]
        count: u32,
        name: String,
    }

    // Build without setting count - should use default
    let props = TestProps::builder().name("test".to_string()).build();

    assert_eq!(props.count, 0);
    assert_eq!(props.name, "test");
}

/// Test that bon's into attribute works with Props.
#[test]
fn test_bon_with_into() {
    #[derive(Builder, Props, Clone, PartialEq)]
    #[props(bon)]
    struct TestProps {
        #[builder(into)]
        name: String,
        count: u32,
    }

    // Build with &str - should auto-convert to String
    let props = TestProps::builder()
        .name("test") // &str, not String
        .count(42)
        .build();

    assert_eq!(props.name, "test");
    assert_eq!(props.count, 42);
}

/// Test the FunctionComponent trait that enables `Component.new()` syntax.
#[test]
fn test_function_component_trait() {
    #[derive(Builder, Props, Clone, PartialEq)]
    #[props(bon)]
    struct TestProps {
        value: i32,
    }

    fn TestComponent(props: TestProps) -> Element {
        rsx! { "{props.value}" }
    }

    // Use the FunctionComponent trait's .new() method
    let props = TestComponent.new().value(42).build();
    assert_eq!(props.value, 42);
}

/// Test memoization with bon-generated props.
#[test]
fn test_memoize_with_bon() {
    #[derive(Builder, Props, Clone, PartialEq, Debug)]
    #[props(bon)]
    struct TestProps {
        value: i32,
    }

    let mut props1 = TestProps::builder().value(1).build();
    let props2 = TestProps::builder().value(1).build();
    let props3 = TestProps::builder().value(2).build();

    // Same value should return true (can be memoized)
    assert!(props1.memoize(&props2));
    // Different value should return false (needs re-render)
    assert!(!props1.memoize(&props3));
}

/// Test that Properties::builder() works correctly.
#[test]
fn test_properties_builder_method() {
    #[derive(Builder, Props, Clone, PartialEq)]
    #[props(bon)]
    struct TestProps {
        value: i32,
    }

    // Use the Properties trait's builder() method
    let props = <TestProps as Properties>::builder().value(42).build();
    assert_eq!(props.value, 42);
}

/// Test with generic types.
#[test]
fn test_bon_with_generics() {
    #[derive(Builder, Props, Clone, PartialEq)]
    #[props(bon)]
    struct TestProps<T: Clone + PartialEq + 'static> {
        value: T,
    }

    let props = TestProps::<i32>::builder().value(42).build();
    assert_eq!(props.value, 42);

    let props = TestProps::<String>::builder()
        .value("hello".to_string())
        .build();
    assert_eq!(props.value, "hello");
}

/// Test with optional fields using Option<T>.
#[test]
fn test_bon_with_option() {
    #[derive(Builder, Props, Clone, PartialEq)]
    #[props(bon)]
    struct TestProps {
        required: i32,
        optional: Option<String>,
    }

    // Build without optional - bon auto-defaults Option<T> to None
    let props = TestProps::builder().required(42).build();
    assert_eq!(props.required, 42);
    assert_eq!(props.optional, None);

    // Build with optional - bon's setter for Option<T> takes T directly
    let props = TestProps::builder()
        .required(42)
        .optional("hello".to_string())
        .build();
    assert_eq!(props.required, 42);
    assert_eq!(props.optional, Some("hello".to_string()));
}
