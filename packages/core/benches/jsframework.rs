#![allow(non_snake_case, non_upper_case_globals)]
//! This benchmark tests just the overhead of Dioxus itself.
//!
//! For the JS Framework Benchmark, both the framework and the browser is benchmarked together. Dioxus prepares changes
//! to be made, but the change application phase will be just as performant as the vanilla wasm_bindgen code. In essence,
//! we are measuring the overhead of Dioxus, not the performance of the "apply" phase.
//!
//!
//! Pre-templates (Mac M1):
//! - 3ms to create 1_000 rows
//! - 30ms to create 10_000 rows
//!
//! Post-templates
//! - 580us to create 1_000 rows
//! - 6.2ms to create 10_000 rows
//!
//! As pure "overhead", these are amazing good numbers, mostly slowed down by hitting the global allocator.
//! These numbers don't represent Dioxus with the heuristic engine installed, so I assume it'll be even faster.

use criterion::{BatchSize, Criterion, criterion_group, criterion_main};
use dioxus::prelude::*;
use dioxus_core::{NoOpMutations, ScopeId};
use rand::prelude::*;
use std::{cell::RefCell, hint::black_box, rc::Rc};

criterion_group!(mbenches, create_rows, js_framework_benchmark_core);
criterion_main!(mbenches);

fn create_rows(c: &mut Criterion) {
    c.bench_function("create rows", |b| {
        let mut dom = VirtualDom::new(synthetic_app);
        dom.rebuild(&mut dioxus_core::NoOpMutations);

        b.iter(|| {
            dom.rebuild(&mut NoOpMutations);
        })
    });
}

fn synthetic_app() -> Element {
    let mut rng = SmallRng::from_os_rng();

    rsx! (
        table {
            tbody {
                for f in 0..10_000_usize {
                    table_row {
                        row_id: f,
                        label: Label::new(&mut rng)
                    }
                }
            }
        }
    )
}

#[derive(PartialEq, Props, Clone, Copy)]
struct SyntheticRowProps {
    row_id: usize,
    label: Label,
}
fn table_row(props: SyntheticRowProps) -> Element {
    let [adj, col, noun] = props.label.0;

    rsx! {
        tr {
            td { class:"col-md-1", "{props.row_id}" }
            td { class:"col-md-1", onclick: move |_| { /* run onselect */ },
                a { class: "lbl", "{adj}" "{col}" "{noun}" }
            }
            td { class: "col-md-1",
                a { class: "remove", onclick: move |_| {/* remove */},
                    span { class: "glyphicon glyphicon-remove remove", aria_hidden: "true" }
                }
            }
            td { class: "col-md-6" }
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
struct Label([&'static str; 3]);

impl Label {
    fn new(rng: &mut SmallRng) -> Self {
        Label([
            ADJECTIVES.choose(rng).unwrap(),
            COLOURS.choose(rng).unwrap(),
            NOUNS.choose(rng).unwrap(),
        ])
    }
}

static ADJECTIVES: &[&str] = &[
    "pretty",
    "large",
    "big",
    "small",
    "tall",
    "short",
    "long",
    "handsome",
    "plain",
    "quaint",
    "clean",
    "elegant",
    "easy",
    "angry",
    "crazy",
    "helpful",
    "mushy",
    "odd",
    "unsightly",
    "adorable",
    "important",
    "inexpensive",
    "cheap",
    "expensive",
    "fancy",
];

static COLOURS: &[&str] = &[
    "red", "yellow", "blue", "green", "pink", "brown", "purple", "brown", "white", "black",
    "orange",
];

static NOUNS: &[&str] = &[
    "table", "chair", "house", "bbq", "desk", "car", "pony", "cookie", "sandwich", "burger",
    "pizza", "mouse", "keyboard",
];

fn js_framework_benchmark_core(c: &mut Criterion) {
    let mut group = c.benchmark_group("js-framework-benchmark core");

    group.bench_function("create 1,000 rows", |b| {
        b.iter_batched(
            JsFrameworkDom::new,
            |mut app| black_box(app.run(1_000)),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("create 10,000 rows", |b| {
        b.iter_batched(
            JsFrameworkDom::new,
            |mut app| black_box(app.run(10_000)),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("replace all rows", |b| {
        b.iter_batched(
            || JsFrameworkDom::with_rows(1_000),
            |mut app| black_box(app.run(1_000)),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("append 1,000 rows", |b| {
        b.iter_batched(
            || JsFrameworkDom::with_rows(1_000),
            |mut app| black_box(app.append(1_000)),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("update every 10th row", |b| {
        b.iter_batched(
            || JsFrameworkDom::with_rows(1_000),
            |mut app| black_box(app.update_every_10th()),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("select row", |b| {
        b.iter_batched(
            || JsFrameworkDom::with_rows(1_000),
            |mut app| black_box(app.select_at(1)),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("swap rows", |b| {
        b.iter_batched(
            || JsFrameworkDom::with_rows(1_000),
            |mut app| black_box(app.swap_rows()),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("remove row", |b| {
        b.iter_batched(
            || JsFrameworkDom::with_rows(1_000),
            |mut app| black_box(app.remove_at(3)),
            BatchSize::SmallInput,
        )
    });

    group.bench_function("clear rows", |b| {
        b.iter_batched(
            || JsFrameworkDom::with_rows(1_000),
            |mut app| black_box(app.clear()),
            BatchSize::SmallInput,
        )
    });

    group.finish();
}

struct JsFrameworkDom {
    dom: VirtualDom,
    controls: Rc<RefCell<Option<Controls>>>,
    generator: RowGenerator,
}

impl JsFrameworkDom {
    fn new() -> Self {
        let controls = Rc::new(RefCell::new(None));
        let generator = RowGenerator::new();
        let props = AppProps {
            controls: controls.clone(),
            generator: generator.clone(),
        };
        let mut dom = VirtualDom::new_with_props(js_framework_app, props);
        dom.rebuild(&mut NoOpMutations);

        Self {
            dom,
            controls,
            generator,
        }
    }

    fn with_rows(count: usize) -> Self {
        let mut app = Self::new();
        app.run(count);
        app
    }

    fn run(&mut self, count: usize) -> usize {
        self.with_runtime(|controls, generator| controls.run(generator, count));
        self.render_and_count()
    }

    fn append(&mut self, count: usize) -> usize {
        self.with_runtime(|controls, generator| controls.append(generator, count));
        self.render_and_count()
    }

    fn update_every_10th(&mut self) -> usize {
        self.with_runtime(|controls, _| controls.update_every_10th());
        self.render_and_count()
    }

    fn select_at(&mut self, index: usize) -> usize {
        self.with_runtime(|controls, _| controls.select_at(index));
        self.render_and_count()
    }

    fn swap_rows(&mut self) -> usize {
        self.with_runtime(|controls, _| controls.swap_rows());
        self.render_and_count()
    }

    fn remove_at(&mut self, index: usize) -> usize {
        self.with_runtime(|controls, _| controls.remove_at(index));
        self.render_and_count()
    }

    fn clear(&mut self) -> usize {
        self.with_runtime(|controls, _| controls.clear());
        self.render_and_count()
    }

    fn render_and_count(&mut self) -> usize {
        self.dom.render_immediate(&mut NoOpMutations);
        self.controls().row_count()
    }

    fn controls(&self) -> Controls {
        self.controls
            .borrow()
            .expect("js-framework-benchmark controls should be initialized after rebuild")
    }

    fn with_runtime<O>(&self, f: impl FnOnce(Controls, &RowGenerator) -> O) -> O {
        let controls = self.controls();
        let generator = &self.generator;
        self.dom
            .runtime()
            .in_scope(ScopeId::APP, || f(controls, generator))
    }
}

#[derive(Clone)]
struct AppProps {
    controls: Rc<RefCell<Option<Controls>>>,
    generator: RowGenerator,
}

impl PartialEq for AppProps {
    fn eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.controls, &other.controls) && self.generator.ptr_eq(&other.generator)
    }
}

#[derive(Clone)]
struct RowGenerator(Rc<RefCell<RowGeneratorState>>);

struct RowGeneratorState {
    rng: SmallRng,
    next_id: usize,
}

impl RowGenerator {
    fn new() -> Self {
        Self(Rc::new(RefCell::new(RowGeneratorState {
            rng: SmallRng::seed_from_u64(0),
            next_id: 1,
        })))
    }

    fn ptr_eq(&self, other: &Self) -> bool {
        Rc::ptr_eq(&self.0, &other.0)
    }

    fn row(&self) -> RowData {
        let mut state = self.0.borrow_mut();
        let adjective = select_random(ADJECTIVES, &mut state.rng);
        let colour = select_random(COLOURS, &mut state.rng);
        let noun = select_random(NOUNS, &mut state.rng);
        let capacity = adjective.len() + colour.len() + noun.len() + 2;
        let mut label = String::with_capacity(capacity);
        label.push_str(adjective);
        label.push(' ');
        label.push_str(colour);
        label.push(' ');
        label.push_str(noun);

        let id = state.next_id;
        state.next_id += 1;

        RowData {
            id,
            label: Signal::new(label),
        }
    }
}

// A native copy of the keyed Dioxus js-framework-benchmark app:
// https://github.com/krausest/js-framework-benchmark/blob/master/frameworks/keyed/dioxus/src/main.rs
// The component and signal structure match the browser implementation, but
// Criterion drives the actions directly and Dioxus writes NoOpMutations so we
// only measure core.
fn js_framework_app(props: AppProps) -> Element {
    let mut rows = use_signal(Vec::<RowData>::new);
    let selected_row: Signal<Option<usize>> = use_signal(|| None);
    #[allow(clippy::redundant_closure)]
    let compare_selected = use_set_compare(move || selected_row());

    *props.controls.borrow_mut() = Some(Controls { rows, selected_row });

    rsx! {
        div { class: "container",
            div { class: "jumbotron",
                div { class: "row",
                    div { class: "col-md-6",
                        h1 { "Dioxus" }
                    }
                    div { class: "col-md-6",
                        div { class: "row",
                            Button {
                                name: "Create 1,000 rows",
                                id: "run",
                                onclick: {
                                    let generator = props.generator.clone();
                                    move |_| randomize_rows(rows, &generator, 1_000)
                                }
                            }
                            Button {
                                name: "Create 10,000 rows",
                                id: "runlots",
                                onclick: {
                                    let generator = props.generator.clone();
                                    move |_| randomize_rows(rows, &generator, 10_000)
                                }
                            }
                            Button {
                                name: "Append 1,000 rows",
                                id: "add",
                                onclick: {
                                    let generator = props.generator.clone();
                                    move |_| add_data(&mut rows.write(), &generator, 1_000)
                                }
                            }
                            Button {
                                name: "Update every 10th row",
                                id: "update",
                                onclick: move |_| update_every_10th(rows)
                            }
                            Button {
                                name: "Clear",
                                id: "clear",
                                onclick: move |_| rows.clear()
                            }
                            Button {
                                name: "Swap rows",
                                id: "swaprows",
                                onclick: move |_| {
                                    if rows.len() > 998 {
                                        rows.write().swap(1, 998);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            table { class: "table table-hover table-striped test-data",
                tbody { id: "tbody",
                    for row in rows.iter() {
                        Row {
                            key: "{row.id}",
                            id: row.id,
                            label: row.label,
                            rows,
                            compare_selected,
                            selected_row
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Row(
    rows: Signal<Vec<RowData>>,
    id: usize,
    label: Signal<String>,
    compare_selected: SetCompare<Option<usize>>,
    mut selected_row: Signal<Option<usize>>,
) -> Element {
    use_drop(move || {
        label.manually_drop();
    });
    let selected = use_set_compare_equal(Some(id), compare_selected);
    rsx! {
        tr { class: if selected() { "danger" },
            td { class: "col-md-1", "{id}" }
            td {
                class: "col-md-4",
                onclick: move |_| selected_row.set(Some(id)),
                a { class: "lbl", {label} }
            }
            td { class: "col-md-1",
                a {
                    class: "remove",
                    onclick: move |_| rows.write().retain(|other_row| other_row.id != id),
                    span {
                        class: "glyphicon glyphicon-remove remove",
                        aria_hidden: "true"
                    }
                }
            }
            td { class: "col-md-6" }
        }
    }
}

#[component]
fn Button(name: String, id: String, onclick: EventHandler) -> Element {
    rsx! {
        div { class: "col-sm-6 smallpad",
            button {
                class: "btn btn-primary btn-block",
                r#type: "button",
                id,
                onclick: move |_| onclick(()),
                "{name}"
            }
        }
    }
}

#[derive(PartialEq, Clone, Copy)]
struct RowData {
    id: usize,
    label: Signal<String>,
}

#[derive(Clone, Copy)]
struct Controls {
    rows: Signal<Vec<RowData>>,
    selected_row: Signal<Option<usize>>,
}

impl Controls {
    fn run(self, generator: &RowGenerator, count: usize) {
        randomize_rows(self.rows, generator, count);
    }

    fn append(mut self, generator: &RowGenerator, count: usize) {
        add_data(&mut self.rows.write(), generator, count);
    }

    fn update_every_10th(self) {
        update_every_10th(self.rows);
    }

    fn select_at(mut self, index: usize) {
        if let Some(row) = self.rows.get(index) {
            self.selected_row.set(Some(row.id));
        }
    }

    fn swap_rows(mut self) {
        if self.rows.len() > 998 {
            self.rows.write().swap(1, 998);
        }
    }

    fn remove_at(mut self, index: usize) {
        let id = self.rows.get(index).map(|row| row.id);
        if let Some(id) = id {
            self.rows.write().retain(|other_row| other_row.id != id);
        }
    }

    fn clear(mut self) {
        self.rows.clear();
    }

    fn row_count(self) -> usize {
        self.rows.len()
    }
}

fn randomize_rows(mut rows: Signal<Vec<RowData>>, generator: &RowGenerator, count: usize) {
    let mut write = rows.write();
    write.clear();
    add_data(&mut write, generator, count);
}

fn add_data(rows: &mut Vec<RowData>, generator: &RowGenerator, count: usize) {
    rows.reserve_exact(count);

    for _ in 0..count {
        rows.push(generator.row());
    }
}

fn update_every_10th(rows: Signal<Vec<RowData>>) {
    for row in rows.iter().step_by(10) {
        *row.label.write_unchecked() += " !!!";
    }
}

fn select_random<'a>(data: &'a [&'a str], rng: &mut SmallRng) -> &'a str {
    data.choose(rng).unwrap()
}
