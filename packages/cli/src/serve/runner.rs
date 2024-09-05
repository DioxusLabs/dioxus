use super::{handle::AppHandle, ServeUpdate};
use crate::{builder::Platform, bundler::AppBundle, cli::serve::ServeArgs, DioxusCrate, Result};
use futures_util::{future::OptionFuture, stream::FuturesUnordered};
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
    pub(crate) fn start() -> Self {
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

        self.running
            .iter_mut()
            .map(|(platform, handle)| async {
                use ServeUpdate::*;
                let platform = *platform;
                tokio::select! {
                    Some(Ok(Some(msg))) = OptionFuture::from(handle.stdout.as_mut().map(|f| f.next_line())) => {
                        StdoutReceived { platform, msg }
                    },
                    Some(Ok(Some(msg))) = OptionFuture::from(handle.stderr.as_mut().map(|f| f.next_line())) => {
                        StderrReceived { platform, msg }
                    },
                    Some(status) = OptionFuture::from(handle.child.as_mut().map(|f| f.wait())) => {
                        tracing::info!("Child process exited with status: {status:?}");
                        match status {
                            Ok(status) => ProcessExited { status, platform },
                            Err(_err) => todo!("handle error in process joining?"),
                        }
                    }
                    else => futures_util::future::pending().await
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

    pub(crate) async fn kill(&mut self, platform: Platform) {
        self.running.remove(&platform);
    }
}
