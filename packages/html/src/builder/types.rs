/// The types of elements that can be built with the `Html` macro.
pub mod WordBreak {
    pub const Normal: &'static str = "normal";
    pub const BreakAll: &'static str = "break-all";
    pub const KeepAll: &'static str = "keep-all";
    pub const Inherit: &'static str = "inherit";
    pub const Initial: &'static str = "initial";
    pub const Revert: &'static str = "revert";
    pub const RevertLayer: &'static str = "revert-layer";
    pub const Unset: &'static str = "unset";
}

fn my_test() {
    let break_type = WordBreak::Normal;
    let break_type = WordBreak::KeepAll;
}
