#![allow(dead_code, unused_imports)]
//! Compile-only check that `Resource` integrates with the `ProjectAwait`
//! family, including the index/map forwarders.

use std::collections::{BTreeMap, HashMap};
use std::future::Future;

use dioxus_hooks::{PendingResource, ResolvedResource};
use dioxus_signals::{
    ProjectAwait, ProjectAwaitExt, ProjectBTreeMap, ProjectHashMap, ProjectOption, ProjectSlice,
};

fn _resource_root_await(r: PendingResource<u32>) -> impl Future<Output = Option<u32>> {
    r.project_future()
}

fn _resolved_await(r: ResolvedResource<u32>) -> impl Future<Output = u32> {
    r.project_future()
}

fn _resource_get_await(r: ResolvedResource<Vec<u32>>) -> impl Future<Output = u32> {
    r.get(0).unwrap().project_future()
}

fn _resource_hashmap_get_await(
    r: ResolvedResource<HashMap<String, u32>>,
) -> impl Future<Output = u32> {
    r.get_unchecked(String::from("k")).project_future()
}

fn _resource_btreemap_get_await(
    r: ResolvedResource<BTreeMap<String, u32>>,
) -> impl Future<Output = u32> {
    r.get_unchecked(String::from("k")).project_future()
}

fn _resource_await_map(r: PendingResource<u32>) -> impl Future<Output = u32> {
    r.project_await_map(|opt: &Option<u32>| opt.as_ref().expect("missing"))
}
