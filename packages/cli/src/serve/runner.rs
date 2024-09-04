use std::collections::HashMap;

use crate::{
    builder::{BuildResult, BuildUpdate, Platform},
    cli::serve::ServeArgs,
    DioxusCrate,
};

use super::ServeUpdate;

pub struct AppRunner {
    /// Ongoing apps running in place
    ///
    /// They might be actively being being, running, or have exited.
    ///
    /// When a new full rebuild occurs, we will keep these requests here
    pub running: HashMap<Platform, BuildResult>,
}

impl AppRunner {
    pub fn start(serve: &ServeArgs, config: &DioxusCrate) -> Self {
        todo!()
    }

    pub async fn wait(&mut self) -> ServeUpdate {
        // // Exits and stdout/stderr
        //         let processes = self.running.iter_mut().filter_map(|(target, request)| {
        //             let Some(child) = request.child else {
        //                 return None;
        //             };

        //             Some(Box::pin(async move {
        //                 //
        //                 (*target, child.wait().await)
        //             }))
        //         });

        //             ((target, exit_status), _, _) = futures_util::future::select_all(processes) => {
        //                 BuildUpdate::ProcessExited { status: exit_status, target_platform: target }
        //             }
        todo!()
    }
}
