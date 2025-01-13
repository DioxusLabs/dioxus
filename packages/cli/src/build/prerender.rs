use anyhow::Context;
use dioxus_cli_config::{server_ip, server_port};
use futures_util::stream::FuturesUnordered;
use futures_util::StreamExt;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::Path,
    time::Duration,
};
use tokio::process::Command;

pub(crate) async fn pre_render_static_routes(server_exe: &Path) -> anyhow::Result<()> {
    // Use the address passed in through environment variables or default to localhost:9999. We need
    // to default to a value that is different than the CLI default address to avoid conflicts
    let ip = server_ip().unwrap_or_else(|| IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)));
    let port = server_port().unwrap_or(9999);
    let fullstack_address = SocketAddr::new(ip, port);
    let address = fullstack_address.ip().to_string();
    let port = fullstack_address.port().to_string();
    // Borrow port and address so we can easily moe them into multiple tasks below
    let address = &address;
    let port = &port;

    tracing::info!("Running SSG at http://{address}:{port}");

    // Run the server executable
    let _child = Command::new(server_exe)
        .env(dioxus_cli_config::SERVER_PORT_ENV, port)
        .env(dioxus_cli_config::SERVER_IP_ENV, address)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let reqwest_client = reqwest::Client::new();
    // Borrow reqwest_client so we only move the reference into the futures
    let reqwest_client = &reqwest_client;

    // Get the routes from the `/static_routes` endpoint
    let mut routes = None;

    // The server may take a few seconds to start up. Try fetching the route up to 5 times with a one second delay
    const RETRY_ATTEMPTS: usize = 5;
    for i in 0..=RETRY_ATTEMPTS {
        let request = reqwest_client
            .post(format!("http://{address}:{port}/api/static_routes"))
            .send()
            .await;
        match request {
            Ok(request) => {
                routes = Some(request
                    .json::<Vec<String>>()
                    .await
                    .inspect(|text| tracing::debug!("Got static routes: {text:?}"))
                    .context("Failed to parse static routes from the server. Make sure your server function returns Vec<String> with the (default) json encoding")?);
                break;
            }
            Err(err) => {
                // If the request fails, try  up to 5 times with a one second delay
                // If it fails 5 times, return the error
                if i == RETRY_ATTEMPTS {
                    return Err(err).context("Failed to get static routes from server. Make sure you have a server function at the `/api/static_routes` endpoint that returns Vec<String> of static routes.");
                }
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    }

    let routes = routes.expect(
        "static routes should exist or an error should have been returned on the last attempt",
    );

    // Create a pool of futures that cache each route
    let mut resolved_routes = routes
        .into_iter()
        .map(|route| async move {
            tracing::info!("Rendering {route} for SSG");
            // For each route, ping the server to force it to cache the response for ssg
            let request = reqwest_client
                .get(format!("http://{address}:{port}{route}"))
                .header("Accept", "text/html")
                .send()
                .await?;
            // If it takes longer than 30 seconds to resolve the route, log a warning
            let warning_task = tokio::spawn({
                let route = route.clone();
                async move {
                    tokio::time::sleep(Duration::from_secs(30)).await;
                    tracing::warn!("Route {route} has been rendering for 30 seconds");
                }
            });
            // Wait for the streaming response to completely finish before continuing. We don't use the html it returns directly
            // because it may contain artifacts of intermediate streaming steps while the page is loading. The SSG app should write
            // the final clean HTML to the disk automatically after the request completes.
            let _html = request.text().await?;

            // Cancel the warning task if it hasn't already run
            warning_task.abort();

            Ok::<_, reqwest::Error>(route)
        })
        .collect::<FuturesUnordered<_>>();

    while let Some(route) = resolved_routes.next().await {
        match route {
            Ok(route) => tracing::debug!("ssg success: {route:?}"),
            Err(err) => tracing::error!("ssg error: {err:?}"),
        }
    }

    tracing::info!("SSG complete");

    drop(_child);

    Ok(())
}
