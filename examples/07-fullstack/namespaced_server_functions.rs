//! This example demonstrates how to define namespaced server functions in Dioxus Fullstack.
//!
//! Namespaced Server Functions allow you to organize your server functions into logical groups,
//! making it possible to reuse groups of functions as a library across different projects.
//!
//! Namespaced server functions are defined as methods on a struct. The struct itself is the "state"
//! of this group of functions, and can hold any data you want to share across the functions.
//!
//! Unlike regular server functions, namespaced server functions are not automatically registered
//! with `dioxus::launch`. You must explicitly mount the server functions to a given route using the
//! `Endpoint::mount` function. From the client, you can then call the functions using regular method
//! call syntax.
//!
//! Namespaces are designed to make server functions easier to modularize and reuse, making it possible
//! to create a publishable library of server functions that other developers can easily integrate into
//! their own Dioxus Fullstack applications.

use dioxus::fullstack::Endpoint;
use dioxus::prelude::*;

fn main() {
    #[cfg(not(feature = "server"))]
    dioxus::launch(app);

    // On the server, we can customize the models and mount the server functions to a specific route.
    // The `.endpoint()` extension method allows you to mount an `Endpoint<T>` to an axum router.
    #[cfg(feature = "server")]
    dioxus::serve(|| async move {
        //
        todo!()
    });
}

// We mount a namespace of server functions to the "/api/dogs" route.
// All calls to `DOGS` from the client will be sent to this route.
static DOGS: Endpoint<PetApi> = Endpoint::new("/api/dogs", || PetApi { pets: todo!() });

/// Our server functions will be associated with this struct.
struct PetApi {
    /// we can add shared state here if we want
    /// e.g. a database connection pool
    ///
    /// Since `PetApi` exists both on the client and server, we need to conditionally include
    /// the database pool only on the server.
    // #[cfg(feature = "server")]
    pets: dashmap::DashMap<String, String>,
}

impl PetApi {
    /// List all the pets in the database.
    // #[get("/")]
    async fn list(&self) -> Result<Vec<String>> {
        Ok(self.pets.iter().map(|entry| entry.key().clone()).collect())
    }

    /// Get the breed of a specific pet by name.
    // #[get("/{name}")]
    async fn get(&self, name: String) -> Result<String> {
        Ok(self
            .pets
            .get(&name)
            .map(|entry| entry.value().clone())
            .or_not_found("pet not found")?)
    }

    /// Add a new pet to the database.
    // #[post("/{name}")]
    async fn add(&self, name: String, breed: String) -> Result<()> {
        self.pets.insert(name, breed);
        Ok(())
    }

    /// Remove a pet from the database.
    // #[delete("/{name}")]
    async fn remove(&self, name: String) -> Result<()> {
        self.pets.remove(&name).or_not_found("pet not found")?;
        Ok(())
    }

    /// Update a pet's name in the database.
    #[put("/{name}")]
    async fn update(&self, name: String, breed: String) -> Result<()> {
        self.pets.insert(breed.clone(), breed);
        Ok(())
    }
}

/// In our app, we can call the namespaced server functions using regular method call syntax, mixing
/// loaders, actions, and other hooks as normal.
fn app() -> Element {
    let pets = use_loader(|| DOGS.list())?;
    let add = use_action(|name, breed| DOGS.add(name, breed));
    let remove = use_action(|name| DOGS.remove(name));
    let update = use_action(|breed| DOGS.update(breed));

    rsx! {
        div {
            h1 { "My Pets" }
            ul {

            }
        }
    }
}
