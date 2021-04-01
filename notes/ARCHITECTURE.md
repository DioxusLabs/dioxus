# Dioxus Architecture

:) 


```rust

let data = use_context();
data.set(abc);

unsafe {
    // data is unsafely aliased
    data.modify(|&mut data| {
        
    })
}

```
