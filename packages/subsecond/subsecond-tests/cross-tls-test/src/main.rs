use std::{cell::Cell, thread, time::Duration};

use cross_tls_crate::get_bar;
use cross_tls_crate_dylib::get_baz;

fn main() {
    dioxus_devtools::connect_subsecond();
    loop {
        dioxus_devtools::subsecond::call(|| {
            use cross_tls_crate::BAR;
            use cross_tls_crate_dylib::BAZ;

            thread_local! {
                pub static FOO: Cell<f32> = const { Cell::new(1.0) };
            }

            println!("Hello world! two: {}", FOO.get());
            get_bar().with(|f| println!("Bar: {:?}", f.borrow()));
            thread::sleep(Duration::from_secs(1));

            FOO.set(2.0);
            get_bar().with(|f| f.borrow_mut().as_mut().unwrap().value = 3.0);
            get_baz().with(|f| f.borrow_mut().as_mut().unwrap().value = 4.0);

            BAR.with_borrow(|f| {
                println!("Bar: {:?}", f);
            });
            BAZ.with_borrow(|f| {
                println!("Baz: {:?}", f);
            });
        });
    }
}
