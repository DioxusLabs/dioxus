use super::{AppHandle, ServeUpdate};
use crate::{bundler::AppBundle, Platform, Result};
use futures_util::{future::OptionFuture, stream::FuturesUnordered};
use std::{collections::HashMap, net::SocketAddr};
use tokio_stream::StreamExt;

pub(crate) struct AppRunner {
    pub(crate) running: HashMap<Platform, AppHandle>,
}

impl AppRunner {
    pub(crate) fn start() -> Self {
        Self {
            running: Default::default(),
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
                    Some(Ok(Some(msg))) = OptionFuture::from(handle.app_stdout.as_mut().map(|f| f.next_line())) => {
                        StdoutReceived { platform, msg }
                    },
                    Some(Ok(Some(msg))) = OptionFuture::from(handle.app_stderr.as_mut().map(|f| f.next_line())) => {
                        StderrReceived { platform, msg }
                    },
                    Some(status) = OptionFuture::from(handle.app_child.as_mut().map(|f| f.wait())) => {
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
    pub(crate) fn open(
        &mut self,
        app: AppBundle,
        devserver_ip: SocketAddr,
        fullstack_address: Option<SocketAddr>,
    ) -> Result<&AppHandle> {
        let platform = app.build.build.platform();

        // Start the new app before we kill the old one to give it a little bit of time
        let handle = AppHandle::start(app, devserver_ip, fullstack_address)?;
        self.kill(platform);
        self.running.insert(platform, handle);

        Ok(self.running.get(&platform).unwrap())
    }

    pub(crate) fn kill(&mut self, platform: Platform) {
        self.running.remove(&platform);
    }

    /// Open an existing app bundle, if it exists
    pub(crate) async fn open_existing(&self) {}
}
