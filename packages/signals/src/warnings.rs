//! Warnings that can be triggered by suspicious usage of signals

use warnings::warning;

/// A warning that is triggered when a copy value is used in a higher scope that it is owned by
#[warning]
pub fn copy_value_hoisted<T: 'static, S: generational_box::Storage<T> + 'static>(
    value: &crate::CopyValue<T, S>,
    caller: &'static std::panic::Location<'static>,
) {
    let origin_scope = value.origin_scope;
    let Some(rt) = dioxus_core::Runtime::try_current() else {
        return;
    };

    if let Some(current_scope) = rt.try_current_scope_id() {
        // If the current scope is a descendant of the origin scope or is the same scope, we don't need to warn
        if origin_scope == current_scope || rt.is_descendant_of(current_scope, origin_scope) {
            return;
        }
        let create_location = value.value.created_at().unwrap();
        let broken_example = include_str!("../docs/hoist/error.rs");
        let fixed_example = include_str!("../docs/hoist/fixed_list.rs");

        // Otherwise, warn that the value is being used in a higher scope and may be dropped
        tracing::warn!(
            r#"A Copy Value created in {origin_scope:?} (at {create_location}). It will be dropped when that scope is dropped, but it was used in {current_scope:?} (at {caller}) which is not a descendant of the owning scope.
This may cause reads or writes to fail because the value is dropped while it still held.

Help:
Copy values (like CopyValue, Signal, Memo, and Resource) are owned by the scope they are created in. If you use the value in a scope that may be dropped after the origin scope,
it is very easy to use the value after it has been dropped. To fix this, you can move the value to the parent of all of the scopes that it is used in.

Broken example ❌:
```rust
{broken_example}
```

Fixed example ✅:
```rust
{fixed_example}
```"#
        );
    }
}

// Include the examples from the warning to make sure they compile
#[test]
#[allow(unused)]
fn hoist() {
    mod broken {
        use dioxus::prelude::*;
        include!("../docs/hoist/error.rs");
    }
    mod fixed {
        use dioxus::prelude::*;
        include!("../docs/hoist/fixed_list.rs");
    }
}
