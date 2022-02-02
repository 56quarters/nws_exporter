// nws_exporter - Prometheus metrics exporter for api.weather.gov
//
// Copyright 2022 Nick Pillitteri
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.
//

use clap::Parser;
use nws_exporter::client::{ClientError, WeatherGovClient};
use nws_exporter::http::RequestContext;
use nws_exporter::metrics::ForecastMetrics;
use reqwest::Client;
use std::error::Error;
use std::io;
use std::net::SocketAddr;
use std::process;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal::unix::{self, SignalKind};
use tracing::{Instrument, Level};

const DEFAULT_LOG_LEVEL: Level = Level::INFO;
const DEFAULT_BIND_ADDR: ([u8; 4], u16) = ([0, 0, 0, 0], 9782);
const DEFAULT_REFERSH_SECS: u64 = 300;
const DEFAULT_TIMEOUT_MILLIS: u64 = 5000;
const DEFAULT_API_URL: &str = "https://api.weather.gov/";

#[derive(Debug, Parser)]
#[clap(name = "nws_exporter", version = clap::crate_version!())]
struct NwsExporterApplication {
    /// NWS weather station ID to fetch forecasts for
    #[clap(long)]
    station: String,

    /// Base URL for the Weather.gov API
    #[clap(long, default_value_t = DEFAULT_API_URL.into())]
    api_url: String,

    /// Logging verbosity. Allowed values are 'trace', 'debug', 'info', 'warn', and 'error'
    /// (case insensitive)
    #[clap(long, default_value_t = DEFAULT_LOG_LEVEL)]
    log_level: Level,

    /// Fetch weather forecasts from the Weather.gov API at this interval, in seconds.
    #[clap(long, default_value_t = DEFAULT_REFERSH_SECS)]
    refresh_secs: u64,

    /// Timeout for fetching weather forecasts from the Weather.gov API, in milliseconds.
    #[clap(long, default_value_t = DEFAULT_TIMEOUT_MILLIS)]
    timeout_millis: u64,

    /// Address to bind to. By default, nws_exporter will bind to public address since
    /// the purpose is to expose metrics to an external system (Prometheus or another
    /// agent for ingestion)
    #[clap(long, default_value_t = DEFAULT_BIND_ADDR.into())]
    bind: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let opts = NwsExporterApplication::parse();
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(opts.log_level)
            .finish(),
    )
    .expect("failed to set tracing subscriber");

    let timeout = Duration::from_millis(opts.timeout_millis);
    let http_client = Client::builder().timeout(timeout).build().unwrap_or_else(|e| {
        tracing::error!(message = "unable to initialize HTTP client", error = %e);
        process::exit(1)
    });

    // Make an initial request to fetch station information. This allows us to verify that the
    // station the user provided is valid and the API is available before starting the HTTP server
    // and running indefinitely.
    let client = WeatherGovClient::new(http_client, &opts.api_url);
    match client.station(&opts.station).await {
        Err(ClientError::InvalidStation(station)) => {
            tracing::error!(message = "invalid station provided", station = %station);
            process::exit(1)
        }
        Err(e) => {
            tracing::warn!(message = "failed to fetch initial station information", error = %e);
        }
        Ok(s) => {
            tracing::debug!(message = "verified station information", station = ?s);
        }
    }

    let station = opts.station.clone();
    let registry = prometheus::default_registry().clone();
    let metrics = ForecastMetrics::new(&registry);
    let mut interval = tokio::time::interval(Duration::from_secs(opts.refresh_secs));

    tokio::spawn(async move {
        tracing::info!(message = "forecast polling started", api_url = %opts.api_url, station = %station);

        loop {
            let _ = interval.tick().await;
            match client
                .observation(&station)
                .instrument(tracing::span!(Level::DEBUG, "nws_observation"))
                .await
            {
                Ok(obs) => {
                    metrics.observe(&obs);
                    tracing::info!(message = "fetched new forecast", observation = %obs.id);
                }
                Err(e) => {
                    tracing::error!(message = "failed to fetch forecast", error = %e);
                }
            }
        }
    });

    let context = Arc::new(RequestContext::new(registry));
    let handler = nws_exporter::http::text_metrics(context);
    let (sock, server) = warp::serve(handler)
        .try_bind_with_graceful_shutdown(opts.bind, async {
            // Wait for either SIGTERM or SIGINT to shutdown
            tokio::select! {
                _ = sigterm() => {}
                _ = sigint() => {}
            }
        })
        .unwrap_or_else(|e| {
            tracing::error!(message = "error binding to address", address = %opts.bind, error = %e);
            process::exit(1)
        });

    tracing::info!(message = "server started", address = %sock);
    server.await;

    tracing::info!("server shutdown");
    Ok(())
}

/// Return after the first SIGTERM signal received by this process
async fn sigterm() -> io::Result<()> {
    unix::signal(SignalKind::terminate())?.recv().await;
    Ok(())
}

/// Return after the first SIGINT signal received by this process
async fn sigint() -> io::Result<()> {
    unix::signal(SignalKind::interrupt())?.recv().await;
    Ok(())
}
