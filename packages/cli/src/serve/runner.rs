use super::{handle::AppHandle, ServeUpdate};
use crate::{
    builder::{AppBundle, Platform},
    cli::serve::ServeArgs,
    DioxusCrate, Result,
};
use futures_util::stream::FuturesUnordered;
use std::{collections::HashMap, net::SocketAddr};
use tokio_stream::StreamExt;

pub(crate) struct AppRunner {
    /// Ongoing apps running in place
    ///
    /// They might be actively being being, running, or have exited.
    ///
    /// When a new full rebuild occurs, we will keep these requests here
    pub(crate) running: HashMap<Platform, AppHandle>,
}

impl AppRunner {
    pub(crate) fn start(_args: &ServeArgs, _krate: &DioxusCrate) -> Self {
        Self {
            running: Default::default(),
        }
    }

    pub(crate) async fn shutdown(&mut self) {
        for (_, mut handle) in self.running.drain() {
            if let Some(mut child) = handle.child.take() {
                let _ = child.kill().await;
            }
        }
    }

    pub(crate) async fn wait(&mut self) -> ServeUpdate {
        // If there are no running apps, we can just return pending to avoid deadlocking
        if self.running.is_empty() {
            return futures_util::future::pending().await;
        }

        self.running.iter_mut().map(|(platform, handle)| async {
            let platform = *platform;
            tokio::select! {
                Ok(Some(msg)) = handle.stdout.as_mut().unwrap().next_line(), if handle.stdout.is_some() => {
                    ServeUpdate::StdoutReceived { platform, msg }
                },
                Ok(Some(msg)) = handle.stderr.as_mut().unwrap().next_line(), if handle.stderr.is_some() => {
                    ServeUpdate::StderrReceived { platform, msg }
                },
                status = handle.child.as_mut().unwrap().wait(), if handle.child.is_some() => {
                    tracing::info!("Child process exited with status: {status:?}");
                    match status {
                        Ok(status) => ServeUpdate::ProcessExited { status, platform },
                        Err(_err) => todo!("handle error in process joining?"),
                    }
                }
            }
        })
        .collect::<FuturesUnordered<_>>()
        .next()
        .await
        .expect("Stream to pending if not empty")
    }

    /// Finally "bundle" this app and return a handle to it
    pub(crate) async fn open(
        &mut self,
        app: AppBundle,
        devserver_ip: SocketAddr,
        fullstack_address: Option<SocketAddr>,
    ) -> Result<&AppHandle> {
        let handle = AppHandle::start(app, devserver_ip, fullstack_address).await?;
        let platform = handle.app.build.platform();

        if let Some(_previous) = self.running.insert(platform, handle) {
            // close the old app, gracefully, hopefully
        }

        Ok(self.running.get(&platform).unwrap())
    }
}
