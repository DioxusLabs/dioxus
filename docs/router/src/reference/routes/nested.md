# Nested Routes

When developing bigger applications we often want to nest routes within each
other. As an example, we might want to organize a settings menu using this
pattern:

```plain
└ Settings
  ├ General Settings (displayed when opening the settings)
  ├ Change Password
  └ Privacy Settings
```

We might want to map this structure to these paths and components:

```plain
/settings          -> Settings { GeneralSettings }
/settings/password -> Settings { PWSettings }
/settings/privacy  -> Settings { PrivacySettings }
```

Nested routes allow us to do this without repeating /settings in every route.

## Nesting

To nest routes, we use the `#[nest("path")]` and `#[end_nest]` attributes.

The path in nest must not:

1. Contain a [Catch All Segment](index.md#catch-all-segments)
2. Contain a [Query Segment](index.md#query-segments)

If you define a dynamic segment in a nest, it will be available to all child routes and layouts.

To finish a nest, we use the `#[end_nest]` attribute or the end of the enum.

```rust, no_run
{{#include ../../../examples/nest.rs:route}}
```
