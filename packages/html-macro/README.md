# html-macro

```rust
use html_macro::*;

fn main () {
    let component = html! { <div id='component'>Some component</div> };

    let text_var = "You can interpolate text variables";

    let html = html! {
       <div onclick=|_ev: web_sys::MouseEvent| {}>
          You can type text right into the elements
          { component }
          { text_var }
       </div>
    };
    println!("{}", node);
}
```
