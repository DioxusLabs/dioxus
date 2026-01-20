# Manganis Asset System Architecture

Manganis provides compile-time asset management through the `asset!()` macro, enabling type-safe asset references with automatic optimization and cache-busting.

## Asset Macro Architecture

### Compile-Time Expansion

```
asset!("/assets/image.png", AssetOptions::image())
    ↓
1. Path Resolution
   - Resolves relative to CARGO_MANIFEST_DIR
   - Validates path exists and stays within crate

2. File Hashing
   - Creates DefaultHasher from path + options + span
   - Produces 16-character hex hash

3. Generate Link Section
   - Creates __ASSETS__{hash} symbol
   - Creates __MANGANIS__{hash} for legacy CLI
   - Both use CBOR serialization via const_serialize

4. Code Generation
   - Emits BundledAsset const with PLACEHOLDER_HASH
   - Creates Asset struct with function pointers
   - Uses volatile reads to prevent optimization
```

### Link Section Generation

Two static link sections for backward compatibility:

1. **Legacy Format** (`__MANGANIS__{hash}`):
   - Uses const_serialize_07
   - For older CLIs (0.7.0-0.7.1)

2. **New Format** (`__ASSETS__{hash}`):
   - Uses const_serialize_08 (CBOR)
   - For dx >= 0.7.2

Both start with `PLACEHOLDER_HASH` sentinel, replaced by CLI during build.

## Asset Types and Options

### Type Hierarchy

```
AssetOptions
├── ImageAssetOptions
│   ├── format: ImageFormat (Png, Jpg, Webp, Avif)
│   ├── size: ImageSize (Manual | Automatic)
│   └── preload: bool
│
├── CssAssetOptions
│   ├── minify: bool (default: true)
│   ├── preload: bool
│   └── static_head: bool
│
├── JsAssetOptions
│   ├── minify: bool (default: true)
│   ├── preload: bool
│   └── static_head: bool
│
├── CssModuleAssetOptions
│   ├── minify: bool
│   └── preload: bool
│
├── FolderAssetOptions
│   └── (no custom options)
│
└── Unknown (generic binary)
```

All options serializable at const-time via `SerializeConst` derive.

### CSS Module Integration

```rust
css_module!(Styles = "/my.module.css", AssetOptions::css_module());

// Generates:
struct Styles {}
impl Styles {
    pub const header: &str = "abc[hash]";  // Unique scoped class
    pub const button: &str = "def[hash]";
}
```

**CSS Identifier Collection**:
- Scans for `.className` and `#idName` patterns
- Converts to snake_case
- Creates ConstStr values that auto-inject stylesheet on deref

## Const Serialization System

### CBOR Format (RFC 8949 Subset)

**Supported Major Types**:
- UnsignedInteger (0)
- NegativeInteger (1)
- Bytes (2)
- String (3)
- Array (4)
- Map (5)

**Not Supported**: Tagged values (6), floating point (7)

### Const Serialization Mechanism

Layout-based binary copying at compile time:

```rust
unsafe trait SerializeConst: Sized {
    const MEMORY_LAYOUT: Layout;
}

enum Layout {
    Enum(EnumLayout),       // repr(C, u8) required
    Struct(StructLayout),   // Field offsets
    Array(ArrayLayout),     // Fixed-size
    Primitive(PrimitiveLayout),
    List(ListLayout),       // Variable length
}
```

**Process**:
1. Calculate total size from MEMORY_LAYOUT
2. Copy bytes from source following layout
3. Apply transformations (endianness)
4. Append to ConstVec buffer

### ConstStr Implementation

```rust
pub struct ConstStr {
    bytes: [MaybeUninit<u8>; 256],  // Fixed 256-byte buffer
    len: u32,
}
```

Used for asset paths and CSS identifiers. Serialized as List layout.

## Asset Resolution Pipeline

### Build-Time Processing (dx CLI)

**Phase 1: Binary Scanning**
```
1. Read compiled binary
2. Find __ASSETS__ symbols via objfile parser
3. Platform-specific methods:
   - Native: object crate symbol tables
   - Windows PE: PDB file parsing
   - WASM: walrus data section parsing
   - Android: NDK handling
```

**Phase 2: Asset Deserialization**
```
For each __ASSETS__{hash}:
1. Read serialized bytes from section
2. Deserialize BundledAsset via const_serialize_08
3. Fall back to const_serialize_07 if fails
4. Extract: absolute_source_path, bundled_path, options
```

**Phase 3: Unique Asset Collection**
- Deduplicate by (absolute_path, options) pair
- Create AssetManifest with unique set

**Phase 4: Asset Optimization**

| Type | Processing |
|------|------------|
| Image | Resize, convert format, optimize |
| CSS | Minify if enabled |
| JS | Minify if enabled |
| Folder | Recursive copy |
| Unknown | Direct copy with optional hash |

**Phase 5: Binary Patching**
```
For each processed asset:
1. Compute final hash (content + options + version)
2. Create new BundledAsset with:
   - bundled_path: "/assets/{output-filename}"
3. Serialize to CBOR
4. Locate __ASSETS__{hash} in binary
5. Overwrite bytes at symbol offset
```

### Runtime Resolution

**Development Mode** (`!is_bundled_app()`):
```
Asset::resolve()
    → Returns bundled().absolute_source_path
    → Browser/native accesses original files
```

**Production Mode** (`is_bundled_app()`):
```
Asset::resolve()
    → Constructs path from:
       * base_path() (e.g., "/app")
       * bundled_path from BundledAsset
    → Returns "/app/assets/{output-filename}"
```

### Platform-Specific Resolution

**Web (WASM)**:
- `resolve_web_asset()`: Fetches via HTTP `fetch()` API
- Supports CORS headers
- Returns `Vec<u8>`

**Desktop**:
- `resolve_asset_path_from_filesystem()`
- Bundle structure varies:
  - macOS: `../Resources/assets/`
  - Linux: `../lib/{product}/assets/`
  - Windows: `assets/` (same directory as exe)

**Android**:
- `to_java_load_asset()`: Uses NDK AssetManager
- Accesses APK's assets/ directory
- Debug fallback: `/data/local/tmp/dx/`

## Hash-Based Cache Busting

### Hash Computation Layers

**INPUT_HASH** (macro generation):
```
= DefaultHasher(span.debug_string + options.string + asset_path)
Used for: __ASSETS__{INPUT_HASH} symbol
```

**CONTENT_HASH** (build time):
```
= Hash(source_contents + applied_options + manganis_version)
Used for: Final output filename
Example: image.png → image-a1b2c3d4e5f6g7h8.webp
```

**CSS_MODULE_HASH**:
```
= Hash(css_module_options + content_hash)
Used for: Scoped CSS identifiers
```

### Filename Generation

```
IMAGE: /assets/photo.png → /assets/photo-{hash}.webp
CSS/JS: /assets/style.css → /assets/style-{hash}.css
FOLDER: /assets (folder) → /assets/ (unchanged)
```

## Key Architectural Patterns

### Const-Time Code Generation
- Uses proc_macro for compile-time expansion
- Leverages const fn exclusively (no allocators)
- Binary layout determined at compile time

### Volatile Reads for Correctness
- `std::ptr::read_volatile()` prevents optimizing away link section reads
- Critical because link section gets patched post-compilation

### Layout-Respecting Serialization
- Types must have well-defined memory layout
- `repr(C, u8)` for enums
- CBOR for variable-length fields

### Dual Path Resolution
- Dev: Returns absolute_source_path
- Prod: Returns bundled_path with base_path
- Same Asset instance works across configurations

### Symbol-Based Discovery
- No manifest file needed
- Binary symbols serve as asset registry
- Scales with multi-crate applications

## Extension Points

### Adding New Asset Types

1. **Create Options Struct**:
```rust
#[derive(SerializeConst, ...)]
pub struct VideoAssetOptions {
    format: VideoFormat,
    preload: bool,
}
```

2. **Add AssetVariant Case**:
```rust
pub enum AssetVariant {
    Video(VideoAssetOptions),
}
```

3. **Implement Builder**:
```rust
pub const fn video() -> AssetOptionsBuilder<VideoAssetOptions> {
    AssetOptionsBuilder::variant(VideoAssetOptions::default())
}
```

4. **Register in CLI** (`assets.rs`):
```rust
AssetVariant::Video(opts) => {
    // Video-specific processing
}
```

### Future Concepts

**IAAC (Infrastructure as Code)**:
```rust
const DATABASE_CONFIG: Asset = asset!("/config/db.yaml");
// CLI extracts, validates, deploys
```

**secret!() Macro**:
```rust
const API_KEY: Secret = secret!("DIOXUS_API_KEY");
// Compile-time validation
// Runtime injection from secure store
```

## Version Compatibility

**Legacy (0.7.0-0.7.1)**:
- const_serialize_07
- Symbol: `__MANGANIS__{hash}`

**Current (0.7.2+)**:
- const_serialize_08 (CBOR)
- Symbol: `__ASSETS__{hash}`

**Migration**: CLI tries new format first, falls back to legacy if deserialization fails or PLACEHOLDER_HASH still present.
