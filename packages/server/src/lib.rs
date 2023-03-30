#[allow(unused)]
use dioxus_core::prelude::*;

mod adapters;
mod server_fn;

pub mod prelude {
    #[cfg(feature = "axum")]
    pub use crate::adapters::axum_adapter::*;
    pub use crate::server_fn::{DioxusServerContext, ServerFn};
    pub use server_fn::{self, ServerFn as _, ServerFnError};
    pub use server_macro::*;
}

#[cfg(feature = "ssr")]
fn dioxus_ssr_html(
    title: &str,
    application_name: &str,
    base_path: Option<&str>,
    head: Option<&str>,
    app: Component,
) -> String {
    let mut vdom = VirtualDom::new(app);
    let _ = vdom.rebuild();
    let renderered = dioxus_ssr::pre_render(&vdom);
    let base_path = base_path.unwrap_or(".");
    let head = head.unwrap_or_default();
    format!(
        r#"
    <!DOCTYPE html>
<html>
<head>
  <title>{title}</title>
  <meta content="text/html;charset=utf-8" http-equiv="Content-Type" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <meta charset="UTF-8" />
  {head}
</head>
<body>
    <div id="main">
    {renderered}
    </div>
         <script type="module">
    import init from "/{base_path}/assets/dioxus/{application_name}.js";
    init("/{base_path}/assets/dioxus/{application_name}_bg.wasm").then(wasm => {{
      if (wasm.__wbindgen_start == undefined) {{
        wasm.main();
      }}
    }});
  </script>
</body>
</html>"#
    )
}
