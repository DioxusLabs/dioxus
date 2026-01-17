# Dioxus Router Architecture

The Dioxus router provides type-safe, declarative routing with support for nested layouts, dynamic parameters, and platform-agnostic navigation.

## Route Definition

### Routable Trait

Routes are defined using Rust enums with the `#[derive(Routable)]` macro:

```rust
#[derive(Routable, Clone, PartialEq, Debug)]
enum Route {
    #[route("/")]
    Home {},
    #[route("/blog/:blog_id")]
    Blog { blog_id: usize },
    #[route("/edit?:blog_id")]
    Edit { blog_id: usize },
    #[route("/hashtag/#:hash")]
    Hash { hash: String },
}
```

The `Routable` trait provides:
- **`const SITE_MAP`**: Static map of all route segments
- **`render(level: usize) -> Element`**: Renders component at nesting level
- **`FromStr` implementation**: Parses URLs into route enums
- **`Display` implementation**: Converts routes to URLs
- **Helper methods**: `is_child_of()`, `parent()`, `static_routes()`

### Route Matching Priority

1. Query routes (`/?:query`)
2. Static routes (`/route`)
3. Dynamic routes (`/:route`)
4. Catch-all routes (`/:..route`)

### Segment Types

- **Static**: Literal path segments like `/blog`
- **Dynamic**: Single parameters like `/:id` (implements `FromRouteSegment`)
- **CatchAll**: Multiple segments like `/:..segments` (implements `FromRouteSegments`)

### Query Parameters

- **Spread query**: `/?:..query` (entire query string, uses `FromQuery`)
- **Segmented query**: `/?:param1&:param2` (individual params, use `FromQueryArgument`)

### Hash Fragments

`/#:hash_segment` (uses `FromHashFragment`)

## Navigation

### RouterContext

Manages navigation state with these core methods:

```rust
impl RouterContext {
    // Navigation
    pub fn push(&self, target: impl Into<NavigationTarget>);
    pub fn replace(&self, target: impl Into<NavigationTarget>);

    // History
    pub fn go_back(&self);
    pub fn go_forward(&self);
    pub fn can_go_back(&self) -> bool;
    pub fn can_go_forward(&self) -> bool;

    // Introspection
    pub fn current<R: Routable>(&self) -> R;
    pub fn full_route_string(&self) -> String;
    pub fn prefix(&self) -> Option<String>;

    // Site map
    pub fn site_map(&self) -> &'static [SiteMapSegment];
}
```

The router context is:
1. Created in `Router` component with `RouterConfig`
2. Provided via Dioxus context to children
3. Uses **signals** for reactive updates
4. Tracks **subscribers** for re-rendering
5. Integrates with **history provider**

### NavigationTarget

```rust
pub enum NavigationTarget<R = String> {
    Internal(R),      // Route parsed by router
    External(String), // External URL (browser handles)
}
```

## Link Component

Declarative navigation:

```rust
Link {
    to: NavigationTarget,
    active_class: Option<String>,  // Applied when matches current
    class: Option<String>,
    new_tab: bool,
    onclick: Option<EventHandler>,
    children: Element,
}
```

**Behavior**:
1. Accesses router without subscribing
2. Checks if target matches current for `active_class`
3. On click (left button, no modifiers):
   - Prevents default
   - Routes via `router.push_any(target)`
   - Executes optional onclick
4. Generates `<a>` tag with `href` for SEO
5. External links use default browser navigation

## Nested Routes

### Layouts and Outlets

```rust
#[derive(Routable)]
enum Route {
    #[nest("/admin")]
        #[layout(AdminLayout)]
            #[route("/")]
            AdminHome {},
            #[route("/users")]
            Users {},
        #[end_layout]
    #[end_nest]

    #[route("/")]
    Home {},
}

#[component]
fn AdminLayout() -> Element {
    rsx! {
        header { "Admin Panel" }
        Outlet::<Route> {}  // Children render here
    }
}
```

### Outlet Component

- Renders child routes at correct nesting depth
- Uses `OutletContext` to track depth level
- Works with layout wrapping via macro

### VirtualDOM Integration

1. `Routable::render(level)` matches route at nesting level
2. Each layout increments nesting level
3. Router changes mark context-accessing components dirty
4. Outlet at correct depth renders matched component

## Route Parameters

### Parameter Traits

| Component | Trait | Example |
|-----------|-------|---------|
| Path segment | `FromRouteSegment` / `ToRouteSegment` | `/users/:id` |
| Multiple segments | `FromRouteSegments` / `ToRouteSegments` | `/files/:..path` |
| Query full | `FromQuery` | `/?:q=search` |
| Query arg | `FromQueryArgument` | `/?page=1&sort=name` |
| Hash | `FromHashFragment` | `/#section-id` |

### Auto-Implementation

Traits auto-implement for types with standard traits:
- `FromRouteSegment`: `FromStr + Default`
- `FromQueryArgument`: `FromStr + Default`
- `FromQuery`: `From<&str>`
- `FromHashFragment`: `FromStr + Default`

### URL Encoding

- Path segments: `PATH_ASCII_SET` (excludes `/`, `?`, `#`)
- Query strings: `QUERY_ASCII_SET` (excludes `&`, `=`)
- Hash fragments: `FRAGMENT_ASCII_SET`
- Auto encode/decode during parse/display

## Router Macro

### Code Generation

`#[derive(Routable)]` generates three implementations:

**1. FromStr Implementation**:
```rust
impl std::str::FromStr for Route {
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // Split URL into path, query, hash
        // Percent-decode segments
        // Try matching against all routes (specificity order)
        // Return error with attempted routes if no match
    }
}
```

**2. Display Implementation**:
```rust
impl std::fmt::Display for Route {
    fn fmt(&self, f: &mut Formatter) -> Result {
        match self {
            Route::Blog { blog_id } => {
                write!(f, "/blog/{}", blog_id)?;
                // Percent-encode as needed
            }
        }
    }
}
```

**3. Routable Trait Implementation**:
```rust
impl Routable for Route {
    const SITE_MAP: &'static [SiteMapSegment] = &[...];

    fn render(&self, level: usize) -> Element {
        match (level, self) {
            (0, Self::Blog { .. }) => rsx! { Layout { Outlet {} } }
            (1, Self::Blog { blog_id }) => rsx! { Blog { blog_id: *blog_id } }
            _ => VNode::empty()
        }
    }
}
```

### Error Type Generation

Each route generates detailed error enum:
```rust
pub enum RouteMatchError {
    Home(HomeParseError),
    Blog(BlogParseError),
    BlogIdParseError(ParseIntError),
}
```

### Segment Parsing

```rust
pub enum RouteSegment {
    Static(String),        // "/blog" -> Static("blog")
    Dynamic(Ident, Type),  // "/:id" -> Dynamic(id, u32)
    CatchAll(Ident, Type), // "/:..paths" -> CatchAll(paths, Vec<String>)
}
```

**Process**:
1. Split URL into path, query, hash
2. Remove trailing slashes
3. Percent-decode components
4. Split path by `/`
5. Classify segments (static/dynamic/catch-all)
6. Match field names to route parameters
7. Validate types exist in struct fields

## History Abstraction

### History Trait

Platform abstraction for navigation:

```rust
pub trait History {
    fn current_route(&self) -> String;
    fn current_prefix(&self) -> Option<String>;

    fn can_go_back(&self) -> bool;
    fn can_go_forward(&self) -> bool;
    fn go_back(&self);
    fn go_forward(&self);

    fn push(&self, route: String);
    fn replace(&self, path: String);
    fn external(&self, url: String) -> bool;

    fn updater(&self, callback: Arc<dyn Fn() + Send + Sync>);
    fn include_prevent_default(&self) -> bool;
}
```

### MemoryHistory

In-memory fallback implementation:

```rust
pub struct MemoryHistory {
    state: RefCell<MemoryHistoryState>,
    base_path: Option<String>,
}

struct MemoryHistoryState {
    current: String,
    history: Vec<String>,   // For back
    future: Vec<String>,    // For forward
}
```

**Behavior**:
- Default starts at `/`
- `push`: Adds current to history, sets new as current, clears future
- `replace`: Only changes current
- `go_back/forward`: Swaps between stacks
- Does not navigate external URLs

**Use Cases**:
- Testing/SSR without browser history
- In-memory fullstack navigation
- Development without browser

### Platform Implementations

**Web**: Browser History API, popstate events
**Desktop**: Platform-specific window/navigation APIs
**LiveView**: WebSocket with server-managed state

## VirtualDOM Integration

1. **Initialization**:
   - `Router` creates `RouterContext` with config
   - Provides to descendants via context
   - Gets current route from history
   - Syncs with history if needed

2. **URL Change**:
   - `RouterContext::push_any(NavigationTarget)` called
   - History provider's method invoked
   - `change_route()` callback fires
   - Subscribers marked dirty

3. **Re-render**:
   - Components using `use_router()`/`use_navigator()` marked dirty
   - `Routable::render(level)` generates new tree
   - Layouts render at nesting levels
   - `Outlet` renders final component

4. **Parameter Passing**:
   - Route parameters part of enum variant
   - Passed as component props
   - Type-safe at compile time

## Key Design Decisions

1. **Trait-Based Extensibility**: Custom types implement parameter traits
2. **Type Safety**: Routes are enums - invalid routes are compile errors
3. **Macro-Driven**: Reduces boilerplate, efficient matching
4. **Platform Abstraction**: History trait enables different renderers
5. **Reactive Updates**: Signals and subscriptions for efficient re-renders
6. **Nested Layouts**: Outlets allow composition without complexity
7. **URL Encoding**: Automatic percent-encoding/decoding
8. **SEO-Friendly**: Links generate real `<a>` tags with hrefs
