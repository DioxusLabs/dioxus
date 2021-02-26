//! An example where the dioxus vdom is running in a native thread, interacting with webview
//! Content is passed from the native thread into the webview
use dioxus_core::prelude::*;

fn main() {
    dioxus_webview::launch(
        // Customize the webview
        |builder| {
            builder
                .title("Test Dioxus App")
                .size(320, 480)
                .resizable(true)
                .debug(true)
        },
        // Props
        (),
        // Draw the root component
        Example,
    )
    .expect("Webview finished");
}

static Example: FC<()> = |ctx, _props| {
    ctx.view(html! {
        <div>
            <div class="flex items-center justify-center flex-col">
                <div class="flex items-center justify-center">
                    <div class="flex flex-col bg-white rounded p-4 w-full max-w-xs">
                        // Title
                        <div class="font-bold text-xl"> "Jon's awesome site!!11" </div>

                        // Subtext / description
                        <div class="text-sm text-gray-500"> "He worked so hard on it :)" </div>

                        <div class="flex flex-row items-center justify-center mt-6">
                            // Main number
                            <div class="font-medium text-6xl">
                                "1337"
                            </div>
                        </div>

                        // Try another
                        <div class="flex flex-row justify-between mt-6">
                            <a href="http://localhost:8080/fib/{}" class="underline">
                                "Legit made my own React"
                            </a>
                        </div>
                    </div>
                </div>
            </div>
        </div>
    })
};
