//! A resource demo: an async "weather fetch" that restarts when its store
//! dependency changes, keeping the stale value visible while reloading.

use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use refract::prelude::*;

/// A minimal timer future: a thread sleeps, then wakes the runtime.
struct Sleep {
    state: Arc<Mutex<(bool, Option<Waker>)>>,
}

fn sleep(duration: Duration) -> Sleep {
    let state = Arc::new(Mutex::new((false, None::<Waker>)));
    let thread_state = state.clone();
    std::thread::spawn(move || {
        std::thread::sleep(duration);
        let mut state = thread_state.lock().unwrap();
        state.0 = true;
        if let Some(waker) = state.1.take() {
            waker.wake();
        }
    });
    Sleep { state }
}

impl Future for Sleep {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let mut state = self.state.lock().unwrap();
        if state.0 {
            Poll::Ready(())
        } else {
            state.1 = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

fn main() {
    let city = Store::new("Oslo".to_string());

    let forecast: Resource<String> = resource(move || {
        // Tracked: `city` is this resource's dependency.
        let city = city.read().clone();
        async move {
            sleep(Duration::from_millis(50)).await;
            format!("{city}: 21°C and sunny")
        }
    });

    let view = el("div").child(dyn_text(move || match &*forecast.read() {
        ResourceState::Pending => "loading...".to_string(),
        ResourceState::Reloading(old) => format!("{old} (refreshing...)"),
        ResourceState::Ready(report) => report.clone(),
    }));

    println!("{}", view.render_to_string());
    run_until_settled(Duration::from_secs(1));
    println!("{}", view.render_to_string());

    // Changing the dependency restarts the future; the stale forecast stays
    // on screen while the new one loads.
    city.set("Reykjavik".to_string());
    println!("{}", view.render_to_string());
    run_until_settled(Duration::from_secs(1));
    println!("{}", view.render_to_string());
}
