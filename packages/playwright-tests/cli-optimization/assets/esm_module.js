// Authored as an ES module. The CLI must emit `<script type="module">` for this
// asset (because `with_module(true)` is set) so the `import.meta.url` reference
// below parses correctly. Setting a global lets the playwright spec assert the
// module actually executed.
const value = "ok-module";
if (typeof import.meta.url === "string") {
    window.__esm_module_value = value;
}
