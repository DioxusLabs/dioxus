use dioxus::prelude::*;
use dioxus_desktop::window;
use serde::Deserialize;

#[path = "./utils.rs"]
mod utils;

pub fn main() {
    #[cfg(not(windows))]
    utils::check_app_exits(app);
}

static EVALS_RECEIVED: GlobalSignal<usize> = Signal::global(|| 0);
static EVALS_RETURNED: GlobalSignal<usize> = Signal::global(|| 0);

fn app() -> Element {
    // Double 100 values in the value
    use_future(|| async {
        let mut eval = document::eval(
            r#"for (let i = 0; i < 100; i++) {
            let value = await dioxus.recv();
            dioxus.send(value*2);
        }"#,
        );
        todo!("Fix eval tests")
        // for i in 0..100 {
        //     eval.send(serde_json::Value::from(i)).unwrap();
        //     let value = eval.recv().await.unwrap();
        //     assert_eq!(value, serde_json::Value::from(i * 2));
        //     EVALS_RECEIVED.with_mut(|x| *x += 1);
        // }
    });

    // Make sure returning no value resolves the future
    use_future(|| async {
        let eval = document::eval(r#"return;"#);

        eval.await.unwrap();
        EVALS_RETURNED.with_mut(|x| *x += 1);
    });

    // Return a value from the future
    use_future(|| async {
        let eval = document::eval(
            r#"
        return [1, 2, 3];
        "#,
        );

        assert_eq!(
            Vec::<i32>::deserialize(&eval.await.unwrap()).unwrap(),
            vec![1, 2, 3]
        );
        EVALS_RETURNED.with_mut(|x| *x += 1);
    });

    use_memo(|| {
        println!("expected 100 evals received found {}", EVALS_RECEIVED());
        println!("expected 2 eval returned found {}", EVALS_RETURNED());
        if EVALS_RECEIVED() == 100 && EVALS_RETURNED() == 2 {
            window().close();
        }
    });

    VNode::empty()
}
