// use anyhow::Result;
// use dioxus::{fullstack::ServerFn, prelude::*};

// #[cfg(feature = "server")]
// static DB: ServerState<rusqlite::Connection> = ServerState::new(|| {
//     let conn = rusqlite::Connection::open("hotdogdb/hotdog.db").expect("Failed to open database");

//     conn.execute_batch(
//         "CREATE TABLE IF NOT EXISTS dogs (
//             id INTEGER PRIMARY KEY,
//             url TEXT NOT NULL
//         );",
//     )
//     .unwrap();

//     conn
// });

// // #[server]
// // pub async fn do_thing(abc: i32, def: String) -> Result<String> {
// //     Ok("Hello from the backend!".to_string())
// // }

// pub async fn do_thing_expanded() -> Result<String> {
//     struct DoThingExpandedArgs {
//         abc: i32,
//         def: String,
//     }

//     impl ServerFn for DoThingExpandedArgs {
//         const PATH: &'static str;

//         type Protocol;

//         type Output;

//         fn run_body(
//             self,
//         ) -> impl std::prelude::rust_2024::Future<
//             Output = std::result::Result<Self::Output, Self::Error>,
//         > + Send {
//             todo!()
//         }
//     }

//     #[cfg(feature = "server")]
//     {
//         todo!()
//     }

//     #[cfg(not(feature = "server"))]
//     {
//         Ok("Hello from the backend!".to_string())
//     }
// }

// #[get("/api/dogs")]
// pub async fn list_dogs() -> Result<Vec<(usize, String)>> {
//     Ok(DB
//         .prepare("SELECT id, url FROM dogs ORDER BY id DESC LIMIT 10")?
//         .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
//         .collect::<Result<Vec<(usize, String)>, rusqlite::Error>>()?)
// }

// #[delete("/api/dogs/{id}")]
// pub async fn remove_dog(id: usize) -> Result<()> {
//     DB.execute("DELETE FROM dogs WHERE id = ?1", [&id])?;
//     Ok(())
// }

// #[post("/api/dogs")]
// pub async fn save_dog(image: String) -> Result<()> {
//     DB.execute("INSERT INTO dogs (url) VALUES (?1)", [&image])?;
//     Ok(())
// }

// #[layer("/admin-api/")]
// pub async fn admin_layer(request: &mut Request<()>) -> Result<()> {
//     todo!();
// }

// #[middleware("/")]
// pub async fn logging_middleware(request: &mut Request<()>) -> Result<()> {
//     todo!();
//     Ok(())
// }

// #[middleware("/admin-api/")]
// pub async fn admin_middleware(request: &mut Request<()>) -> Result<()> {
//     if request
//         .headers()
//         .get("Authorization")
//         .and_then(|h| h.to_str().ok())
//         != Some("Bearer admin-token")
//     {
//         todo!("unauthorizeda");
//     }

//     Ok(())
// }
