#[allow(unused)]
use dioxus_core::prelude::*;

mod adapters;
#[cfg(feature = "ssr")]
mod serve;
mod server_fn;

pub mod prelude {
    #[cfg(feature = "axum")]
    pub use crate::adapters::axum_adapter::*;
    #[cfg(feature = "salvo")]
    pub use crate::adapters::salvo_adapter::*;
    #[cfg(feature = "warp")]
    pub use crate::adapters::warp_adapter::*;
    #[cfg(feature = "ssr")]
    pub use crate::serve::ServeConfig;
    pub use crate::server_fn::{DioxusServerContext, ServerFn};
    pub use server_fn::{self, ServerFn as _, ServerFnError};
    pub use server_macro::*;
}

#[cfg(feature = "ssr")]
fn dioxus_ssr_html<P: 'static + Clone>(cfg: &serve::ServeConfig<P>) -> String {
    use prelude::ServeConfig;

    let ServeConfig {
        app,
        application_name,
        base_path,
        head,
        props,
        ..
    } = cfg;

    let application_name = application_name.unwrap_or("dioxus");

    let mut vdom = VirtualDom::new_with_props(*app, props.clone());
    let _ = vdom.rebuild();
    let renderered = dioxus_ssr::pre_render(&vdom);
    let base_path = base_path.unwrap_or(".");
    let head = head.unwrap_or(
        r#"<title>Dioxus Application</title>
  <meta content="text/html;charset=utf-8" http-equiv="Content-Type" />
  <meta name="viewport" content="width=device-width, initial-scale=1" />
  <meta charset="UTF-8" />"#,
    );
    format!(
        r#"
    <!DOCTYPE html>
<html>
<head>
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
