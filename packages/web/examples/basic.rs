//! basic example that renders a simple domtree to the page :)
//!
//!
//!
use dioxus_core::prelude::bumpalo::Bump;
use dioxus_core::prelude::*;
use dioxus_web::*;

fn main() {
    wasm_logger::init(wasm_logger::Config::new(log::Level::Debug));
    log::debug!("Hello world, from the app");
    WebsysRenderer::simple_render(html! {

        // Body
        <div class="flex items-center justify-center flex-col">
            <div class="flex items-center justify-center">
                <div class="flex flex-col bg-white rounded p-4 w-full max-w-xs">
                    // Title
                    <div class="font-bold text-xl">
                        // {format!("Fibonacci Calculator: n = {}",n)}
                        "Fibonacci Calculator: n = {}"
                    </div>

                    // Subtext / description
                    <div class="text-sm text-gray-500">
                        // {format!("Calculated in {} nanoseconds",duration)}
                        // {format!("Calculated in {} nanoseconds",duration)}
                        "Calculated in {} nanoseconds"
                    </div>

                    <div class="flex flex-row items-center justify-center mt-6">
                        // Main number
                        <div class="font-medium text-6xl">
                            // {format!("{}",fib_n)}
                        </div>
                    </div>

                    // Try another
                    <div class="flex flex-row justify-between mt-6">
                        // <a href=format!("http://localhost:8080/fib/{}", other_fib_to_try) class="underline">
                            "Click to try another number"
                            // {"Click to try another number"}
                        // </a>
                    </div>
                </div>
            </div>
        </div>

    });
}
