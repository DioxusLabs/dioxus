//! This module contains utilities renderers use to integrate with the launch function.

/// A marker trait for platform configs. We use this marker to
/// make sure that the user doesn't accidentally pass in a config
/// builder instead of the config
pub trait LaunchConfig: 'static {}

impl LaunchConfig for () {}
