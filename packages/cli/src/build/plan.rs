use crate::{BuildArgs, BuildRequest, Platform};

/// Actualize the build args into a build plan
///
/// This applies the arguments of BuildArgs into a list of targets
pub struct BuildPlan {
    pub builds: Vec<BuildRequest>,
}

impl BuildPlan {
    pub fn new(args: BuildArgs) -> Self {
        todo!()
    }

    pub fn client_platform(&self) -> Platform {
        todo!()
    }
}
