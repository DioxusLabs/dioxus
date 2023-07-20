# Defining Routes

When creating a [`Routable`] enum, we can define routes for our application using the `route("path")` attribute.

## Route Segments

Each route is made up of segments. Most segments are separated by `/` characters in the path.

There are four fundamental types of segments:

1. [Static segments](#static-segments) are fixed strings that must be present in the path.
2. [Dynamic segments](#dynamic-segments) are types that can be parsed from a segment.
3. [Catch-all segments](#catch-all-segments) are types that can be parsed from multiple segments.
4. [Query segments](#query-segments) are types that can be parsed from the query string.

Routes are matched:

- First, from most specific to least specific (Static then Dynamic then Catch All) (Query is always matched)
- Then, if multiple routes match the same path, the order in which they are defined in the enum is followed.

## Static segments

Fixed routes match a specific path. For example, the route `#[route("/about")]` will match the path `/about`.

```rust, no_run
{{#include ../../../examples/static_segments.rs:route}}
```

## Dynamic Segments

Dynamic segments are in the form of `:name` where `name` is
the name of the field in the route variant. If the segment is parsed
successfully then the route matches, otherwise the matching continues.

The segment can be of any type that implements `FromStr`.

```rust, no_run
{{#include ../../../examples/dynamic_segments.rs:route}}
```

## Catch All Segments

Catch All segments are in the form of `:...name` where `name` is the name of the field in the route variant. If the segments are parsed successfully then the route matches, otherwise the matching continues.

The segment can be of any type that implements `FromSegments`. (Vec<String> implements this by default)

Catch All segments must be the _last route segment_ in the path (query segments are not counted) and cannot be included in nests.

```rust, no_run
{{#include ../../../examples/catch_all_segments.rs:route}}
```

## Query Segments

Query segments are in the form of `?:name` where `name` is the name of the field in the route variant.

Unlike [Dynamic Segments](#dynamic-segments) and [Catch All Segments](#catch-all-segments), parsing a Query segment must not fail.

The segment can be of any type that implements `FromQuery`.

Query segments must be the _after all route segments_ and cannot be included in nests.

```rust, no_run
{{#include ../../../examples/query_segments.rs:route}}
```
