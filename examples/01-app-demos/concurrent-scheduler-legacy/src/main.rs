use async_std::task::sleep;
use dioxus::prelude::*;
use std::time::Duration;
use web_time::Instant;

#[cfg(feature = "web")]
use wasm_bindgen::prelude::*;

#[cfg(feature = "web")]
#[wasm_bindgen(inline_js = r#"
let installed = false;

function setText(id, text) {
    const node = document.getElementById(id);
    if (node) {
        node.textContent = text;
    }
}

export function installTriangleProbe() {
    if (installed) {
        return;
    }
    installed = true;

    let frames = 0;
    let jankFrames = 0;
    let maxGap = 0;
    let last = performance.now();

    function resetFrameStats() {
        frames = 0;
        jankFrames = 0;
        maxGap = 0;
        last = performance.now();
        setText("urgent-ticks", "0");
        setText("last-gap", "0ms");
        setText("worst-gap", "0ms");
        setText("jank-frames", "0");
    }

    function tick(now) {
        const gap = now - last;
        last = now;
        frames += 1;

        if (gap > maxGap) {
            maxGap = gap;
        }
        if (gap > 80) {
            jankFrames += 1;
        }

        setText("urgent-ticks", String(frames));
        setText("last-gap", `${Math.round(gap)}ms`);
        setText("worst-gap", `${Math.round(maxGap)}ms`);
        setText("jank-frames", String(jankFrames));

        requestAnimationFrame(tick);
    }

    requestAnimationFrame((now) => {
        last = now;
        requestAnimationFrame(tick);
    });

    setTimeout(resetFrameStats, 400);
}
"#)]
extern "C" {
    #[wasm_bindgen(js_name = installTriangleProbe)]
    fn install_triangle_probe();
}

const ROOT_SIZE: f64 = 720.0;
const TARGET_SIZE: f64 = 8.0;
const DOT_SIZE: f64 = 10.0;
const DOT_COUNT: usize = 2_187;
const DOT_WORK: u32 = 18_000;

fn main() {
    dioxus::launch(app);
}

fn app() -> Element {
    #[cfg(feature = "web")]
    install_triangle_probe();

    let mut elapsed_ms = use_signal(|| 0_u32);
    let mut seconds = use_signal(|| 0_u32);

    use_future(move || async move {
        let started = Instant::now();
        let mut last_second = 0;

        loop {
            sleep(Duration::from_millis(16)).await;

            let elapsed = started.elapsed().as_millis() as u32;
            elapsed_ms.set(elapsed);

            let next_second = elapsed / 1_000;
            if next_second != last_second {
                last_second = next_second;
                seconds.set(next_second);
            }
        }
    });

    let scale = scale_for_elapsed(elapsed_ms());

    rsx! {
        style { {STYLE} }
        main {
            class: "shell",
            section {
                class: "control-band",
                div {
                    class: "title-block",
                    h1 { "Stack Triangle" }
                    p { "Same Sierpinski workload on Dioxus 0.6.3 render_immediate." }
                }

                div {
                    class: "stats",
                    Metric { label: "Dots", value: DOT_COUNT.to_string(), value_id: "dot-count" }
                    Metric { label: "Dot work", value: DOT_WORK.to_string() }
                    Metric { label: "Second", value: seconds().to_string(), value_id: "second-count" }
                    Metric { label: "Scale", value: format!("{scale:.3}"), value_id: "scale-value" }
                    Metric { label: "Animation lane", value: "Default".to_string() }
                    Metric { label: "Text lane", value: "Default".to_string() }
                }

                div {
                    class: "stats",
                    Metric { label: "Fiber work", value: "n/a".to_string(), value_id: "fiber-work" }
                    Metric { label: "Fiber commits", value: "n/a".to_string(), value_id: "fiber-commits" }
                    Metric { label: "Fiber yields", value: "n/a".to_string(), value_id: "fiber-yields" }
                    Metric { label: "Frames", value: "0".to_string(), value_id: "urgent-ticks" }
                    Metric { label: "Worst gap", value: "0ms".to_string(), value_id: "worst-gap" }
                    Metric { label: "Jank >80ms", value: "0".to_string(), value_id: "jank-frames" }
                }
            }

            section {
                class: "triangle-stage",
                div {
                    id: "triangle-layer",
                    class: "triangle-layer",
                    style: "transform: scale({scale});",
                    Triangle {
                        x: 700.0,
                        y: 360.0,
                        size: ROOT_SIZE,
                        seconds: seconds(),
                    }
                }
            }
        }
    }
}

#[component]
fn Triangle(x: f64, y: f64, size: f64, seconds: u32) -> Element {
    if size <= TARGET_SIZE {
        return rsx! {
            Dot { x, y, seconds }
        };
    }

    let child_size = size / 2.0;
    rsx! {
        Triangle {
            x,
            y: y - child_size / 2.0,
            size: child_size,
            seconds,
        }
        Triangle {
            x: x - child_size,
            y: y + child_size / 2.0,
            size: child_size,
            seconds,
        }
        Triangle {
            x: x + child_size,
            y: y + child_size / 2.0,
            size: child_size,
            seconds,
        }
    }
}

#[component]
fn Dot(x: f64, y: f64, seconds: u32) -> Element {
    let checksum = expensive_dot_value((x as u32).wrapping_mul(31) ^ y as u32, seconds);
    let left = x - DOT_SIZE / 2.0;
    let top = y - DOT_SIZE / 2.0;
    let color = checksum % 120;

    rsx! {
        div {
            class: "dot",
            style: "left: {left}px; top: {top}px; --hue: {color};",
            "{seconds % 10}"
        }
    }
}

#[component]
fn Metric(label: &'static str, value: String, value_id: Option<&'static str>) -> Element {
    rsx! {
        div { class: "metric",
            span { "{label}" }
            strong { id: value_id, "{value}" }
        }
    }
}

fn expensive_dot_value(seed: u32, seconds: u32) -> u32 {
    let mut value = seed ^ seconds.wrapping_mul(1_013_904_223);
    for step in 0..DOT_WORK {
        value = value.rotate_left(5)
            ^ value.wrapping_mul(747_796_405)
            ^ step.wrapping_mul(2_891_336_453);
    }
    value
}

fn scale_for_elapsed(elapsed_ms: u32) -> f64 {
    let phase = elapsed_ms as f64 / 520.0;
    0.78 + phase.sin() * 0.18
}

const STYLE: &str = r#"
html, body, #main {
    margin: 0;
    min-height: 100%;
    font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
    background: #f4f7f5;
    color: #17211d;
}

.shell {
    min-height: 100vh;
    display: grid;
    grid-template-rows: auto 1fr;
}

.control-band {
    position: sticky;
    top: 0;
    z-index: 2;
    display: grid;
    grid-template-columns: minmax(260px, 0.7fr) minmax(520px, 1fr) minmax(520px, 1fr);
    gap: 18px;
    align-items: center;
    padding: 16px 20px;
    border-bottom: 1px solid #c9d5d0;
    background: rgba(244, 247, 245, 0.96);
    backdrop-filter: blur(14px);
}

.title-block h1 {
    margin: 0;
    font-size: 22px;
    font-weight: 760;
}

.title-block p {
    margin: 4px 0 0;
    color: #596b63;
    font-size: 13px;
    line-height: 1.35;
}

.stats {
    display: grid;
    grid-template-columns: repeat(3, minmax(0, 1fr));
    gap: 8px;
}

.metric {
    min-width: 0;
    border: 1px solid #d0dbd7;
    background: #ffffff;
    border-radius: 6px;
    padding: 8px 10px;
}

.metric span {
    display: block;
    color: #63756e;
    font-size: 11px;
    line-height: 1.2;
}

.metric strong {
    display: block;
    margin-top: 3px;
    font-size: 16px;
    line-height: 1.1;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
}

.triangle-stage {
    position: relative;
    overflow: hidden;
    min-height: 720px;
    background:
        linear-gradient(90deg, rgba(30, 87, 80, 0.06) 1px, transparent 1px),
        linear-gradient(0deg, rgba(30, 87, 80, 0.05) 1px, transparent 1px),
        #eef3f0;
    background-size: 48px 48px;
}

.triangle-layer {
    position: relative;
    width: 1400px;
    height: 760px;
    margin: 0 auto;
    transform-origin: 50% 48%;
    will-change: transform;
}

.dot {
    position: absolute;
    width: 10px;
    height: 10px;
    border-radius: 50%;
    box-sizing: border-box;
    display: grid;
    place-items: center;
    border: 1px solid hsl(calc(168 + var(--hue)), 52%, 31%);
    background: hsl(calc(158 + var(--hue)), 54%, 84%);
    color: #17211d;
    font-size: 7px;
    line-height: 1;
    font-variant-numeric: tabular-nums;
}

@media (max-width: 1120px) {
    .control-band {
        grid-template-columns: 1fr;
    }

    .stats {
        grid-template-columns: repeat(2, 1fr);
    }

    .triangle-layer {
        margin-left: 50%;
        transform-origin: 0 48%;
    }
}
"#;
