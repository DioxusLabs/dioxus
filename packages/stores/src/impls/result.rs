//! All `Store<Result<T, E>, _>` projection methods live on the
//! [`ProjectResult`](crate::ProjectResult) trait in `project.rs`. `Store`
//! implements [`Project`](crate::Project), so every trait method is callable
//! directly on `Store<Result<T, E>, _>`.
