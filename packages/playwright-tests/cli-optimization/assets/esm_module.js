// No top-level import/export — auto-detection alone would classify this as a
// classic script. `with_module(true)` on the asset overrides that and forces
// the CLI to emit `<script type="module">` and run esbuild with the ESM
// pipeline. The playwright spec asserts both that the global is set and that
// the script tag actually has `type="module"`.
window.__esm_module_value = "ok-module";
