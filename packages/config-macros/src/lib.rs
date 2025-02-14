/// A macro for deciding whether or not to split the wasm bundle.
/// Used by the internal router-macro code. The contents here are considered to be semver exempt.
///
/// Only on wasm with the wasm-split feature will we prefer the `maybe_wasm_split` variant that emits
/// the "lefthand" tokens. Otherwise, we emit the non-wasm_split tokens
#[doc(hidden)]
#[cfg(all(feature = "wasm-split", target_arch = "wasm32"))]
#[macro_export]
macro_rules! maybe_wasm_split {
    (
        if wasm_split {
            $left:tt
        } else {
            $right:tt
        }
    ) => {
        $left
    };
}

/// A macro for deciding whether or not to split the wasm bundle.
/// Used by the internal router-macro code. The contents here are considered to be semver exempt.
///
/// Only on wasm with the wasm-split feature will we prefer the `maybe_wasm_split` variant that emits
/// the "lefthand" tokens. Otherwise, we emit the non-wasm_split tokens
#[doc(hidden)]
#[cfg(any(not(feature = "wasm-split"), not(target_arch = "wasm32")))]
#[macro_export]
macro_rules! maybe_wasm_split {
    (
        if wasm_split {
            $left:tt
        } else {
            $right:tt
        }
    ) => {
        $right
    };
}
