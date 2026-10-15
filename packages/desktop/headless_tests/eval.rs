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
static HEAD_SCRIPTS_READY: GlobalSignal<bool> = Signal::global(|| false);

fn app() -> Element {
    // A head script must run before an eval from the same render
    use_future(|| async {
        let ready = document::eval(
            r#"return !!window.__inline_head_ready && !!window.__external_head_ready;"#,
        )
        .join::<bool>()
        .await
        .unwrap();
        assert!(ready, "document::eval ran before the head scripts");
        HEAD_SCRIPTS_READY.with_mut(|x| *x = true);
    });

    // Double 100 values in the value
    use_future(|| async {
        let mut eval = document::eval(
            r#"for (let i = 0; i < 100; i++) {
            let value = await dioxus.recv();
            dioxus.send(value*2);
        }"#,
        );
        for i in 0..100 {
            eval.send(i).unwrap();
            let value: i32 = eval.recv().await.unwrap();
            assert_eq!(value, i * 2);
            EVALS_RECEIVED.with_mut(|x| *x += 1);
        }
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
        if EVALS_RECEIVED() == 100 && EVALS_RETURNED() == 2 && HEAD_SCRIPTS_READY() {
            window().close();
        }
    });

    rsx! {
        document::Script { "window.__inline_head_ready = true;" }
        document::Script { src: "data:text/javascript;base64,d2luZG93Ll9fZXh0ZXJuYWxfaGVhZF9yZWFkeT10cnVlOw==" }
        // Links that never load must not block later evals.
        document::Link { rel: "stylesheet" }
        document::Link { rel: "stylesheet", href: "data:text/css,", disabled: true }
        // A script that fails to load must resolve the gate too.
        document::Script { src: "dioxus://index.html/__missing_head_script.js" }
    }
}
