//! A simple counters example that stores a list of items in a vec and then iterates over them.

use dioxus::prelude::*;

const STYLE: Asset = asset!("/examples/assets/counter.css");

fn main() {
    dioxus::launch(app);
}

pub struct Test {
    name: String,
    steps: Vec<i32>,
}

impl Test {
    pub fn default() -> Self {
        Self {
            name: "test".to_string(),
            steps: vec![0, 0, 0],
        }
    }

    pub fn get_steps(&self) -> &Vec<i32> {
        &self.steps
    }

    pub fn add_default_step(&mut self) {
        self.steps.push(0);
    }

    pub fn delete_step(&mut self) {
        self.steps.pop();
    }

    pub fn write_step(&mut self, idx: usize, val: i32) {
        self.steps[idx] = val;
    }
}

pub fn MixerRecipeStep(id: usize, mut recipe: Signal<Test>) -> Element {
    let recipe_read = recipe.read();
    //let steps = recipe_read.get_steps();
    //println!("Starting {id}");
    rsx!(
        div {
            label { "{id + 1}."}
        }
        div {
            select {
                value: "Apple", //selected_options.read()[id].to_string(),
                onchange: move |evt| {
                    //selected_options.write()[id] = evt.value().to_string()
                },
                    option { value:"1", "apple" },
                    option { value:"2", "banana" },
                    option { value:"3", "strawberry" },
                    option { value:"4", "yolo" }
                    }
        }
        div {
            label { r#for:"time", "Zeit: " }
        }
        div{
            input { style:"width: 60px",
                r#type: "number",
                    value: "{id}",
                onchange: move |evt|{
                    let val: i32 = evt.value().parse().expect("error");
                    recipe.write().write_step(id, val);
                } }
        }
        div{
            label { "s  " }
        }
        div{
            label { "   Geschwindigkeit" }
        }
        div{
            input { style:"width: 60px",
                r#type: "number",
                    value: "{id}",
                onchange: move |evt|{
                    let val: i32 = evt.value().parse().expect("error");
                    recipe.write().write_step(id, val);
                } }
            }
        div{
            label { "U/min  " }
            }
        div {
            style:"position: center",
            button { "delete" }
        }
    )
}

fn app() -> Element {
    // Store the counters in a signal
    let mut rec = use_signal(|| Test::default());

    // Whenever the counters change, sum them up
    let mut selected_options = use_signal(|| {
        vec![
            "Apple".to_string(),
            "Apple".to_string(),
            "Apple".to_string(),
        ]
    });

    rsx! {

        div { style:"    display: grid;
    grid-template-rows: 10% 90%;
    gap: 10px;
    padding: 30px 30px 0px 30px;
    width: 100%;
    height: 100%;",

        div { id: "controls",
            button { onclick: move |_| {rec.write().add_default_step(); selected_options.write().push("Apple".to_string());}, "Add counter" }
            button { onclick: move |_| { rec.write().delete_step(); selected_options.pop(); }, "Remove counter" }
        }

        div
            {
                    style:"    height: 10rem;
                overflow-y: auto; 
                box-shadow: inset 0px 0px 10px 0px rgba(0, 0, 0, 0.25);
                border-radius: var(--border-radius);",
                div {
                        style:"    display: grid;
                    grid-template-columns: 5% 20% 10% 15% 5% 15% 15% 5% 5%;
                position: relative;
                padding: 10px;
                width: 100%;",
                // Calling `iter` on a Signal<Vec<>> gives you a GenerationalRef to each entry in the vec
                // We enumerate to get the idx of each counter, which we use later to modify the vec
                for (id, _) in rec.read().get_steps().iter().enumerate() {
                // We need a key to uniquely identify each counter. You really shouldn't be using the index, so we're using
                // the counter value itself.
                //
                // If we used the index, and a counter is removed, dioxus would need to re-write the contents of all following
                // counters instead of simply removing the one that was removed
                //
                // You should use a stable identifier for the key, like a unique id or the value of the counter itself
                    {
                        MixerRecipeStep(id,
                            rec
                        )
                    }

                }

                div {}, // placeholder
                div {}, // placeholder
                div {}, // placeholder
                div {style: "padding: 5px",
                    button {
                        onclick: move |_| {

                        },
                        "Press"
                    }
                }
            }

        }
    }
        }
}
