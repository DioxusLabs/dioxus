//! Weather: an async resource that reloads when its lens dependencies
//! change, keeping stale data visible while a new fetch is in flight.

use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use refract::{ResourceState, Ui, dyn_text, el, lens};

struct App {
    city: String,
}

/// A fake network call: completes after being polled `ticks` times.
struct Timer {
    remaining: Rc<Cell<u32>>,
}

impl Future for Timer {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let left = self.remaining.get();
        if left == 0 {
            Poll::Ready(())
        } else {
            self.remaining.set(left - 1);
            // Ask to be polled again, like a real timer would on expiry.
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

fn fetch_weather(city: String) -> impl Future<Output = String> {
    let timer = Timer {
        remaining: Rc::new(Cell::new(2)),
    };
    async move {
        timer.await;
        format!("{city}: 21°C and sunny")
    }
}

fn main() {
    let mut ui = Ui::new(App {
        city: "Oslo".to_string(),
    });
    let city = lens!(App => 0: city);

    let weather = ui.resource(move |ctx| {
        // Tracked: the resource restarts when `city` changes.
        let city = ctx.get(city).clone();
        fetch_weather(city)
    });

    let root = ui.mount(
        el("div").child(dyn_text(move |ctx| match ctx.read_resource(weather) {
            ResourceState::Pending => "loading...".to_string(),
            ResourceState::Ready(report) => report.clone(),
            ResourceState::Reloading(stale) => format!("{stale} (refreshing...)"),
        })),
    );

    println!("{}", ui.render_to_string(root));
    ui.run_until_settled();
    println!("{}", ui.render_to_string(root));

    // Change the city: the in-flight future is cancelled, stale data stays
    // visible while the new fetch runs.
    ui.with(|ctx| *ctx.write(city) = "Reykjavik".to_string());
    println!("{}", ui.render_to_string(root));
    ui.run_until_settled();
    println!("{}", ui.render_to_string(root));
}
