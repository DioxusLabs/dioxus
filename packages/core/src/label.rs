use std::any::Any;
use dyn_hash::DynHash;

/// Type erased value that acts as a label for a hook
pub trait HookLabel: DynEq + DynHash + Send + Sync{}
impl<T: DynEq + DynHash + Send + Sync> HookLabel for T {}
pub trait DynEq: Any {
	fn dyn_eq(&self, other: &dyn DynEq) -> bool;
}

impl PartialEq for dyn HookLabel {
	fn eq(&self, other: &dyn HookLabel) -> bool {
		(self as &dyn DynEq).dyn_eq(other as &dyn DynEq)
	}
}
impl Eq for dyn HookLabel {}
dyn_hash::hash_trait_object!(HookLabel);

impl<T: Any + Eq + 'static> DynEq for T {
	fn dyn_eq(&self, other: &dyn DynEq) -> bool {
		(other as &dyn Any).downcast_ref::<T>().filter(|other| PartialEq::eq(self, other)).is_some()
	}
}
