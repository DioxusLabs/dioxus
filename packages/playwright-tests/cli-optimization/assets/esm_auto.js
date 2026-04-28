// No `with_module(true)` is set on the asset — the CLI must detect this file
// as ESM from its source (top-level `export`) and emit `<script type="module">`
// automatically. The `import.meta.url` reference would be a syntax error if the
// script tag were a classic <script>.
export const value = "ok-auto";
window.__esm_auto_value = value;
window.__esm_auto_meta = typeof import.meta.url;
