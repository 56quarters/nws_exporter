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

use axum::routing::get;
use axum::Router;
use clap::Parser;
use nws_exporter::client::{ClientError, NwsClient};
use nws_exporter::http::RequestState;
use nws_exporter::metrics::ForecastMetrics;
use prometheus_client::registry::Registry;
use reqwest::Client;
use std::error::Error;
use std::io;
use std::net::SocketAddr;
use std::process;
use std::sync::Arc;
use std::time::Duration;
use tower_http::trace::TraceLayer;
use tracing::{Instrument, Level};

const DEFAULT_LOG_LEVEL: Level = Level::INFO;
const DEFAULT_BIND_ADDR: ([u8; 4], u16) = ([0, 0, 0, 0], 9782);
const DEFAULT_REFERSH_SECS: u64 = 300;
const DEFAULT_TIMEOUT_MILLIS: u64 = 5000;
const DEFAULT_API_URL: &str = "https://api.weather.gov/";

/// Export National Weather Service forecasts as Prometheus metrics
#[derive(Debug, Parser)]
#[clap(name = "nws_exporter", version = clap::crate_version!())]
struct NwsExporterApplication {
    /// NWS weather station ID to fetch forecasts for. Must be specified at least once and
    /// may be used multiple times (separated by spaces) to fetch forecasts for multiple NWS
    /// stations
    #[arg(required = true)]
    station: Vec<String>,

    /// Base URL for the Weather.gov API
    #[arg(long, default_value_t = DEFAULT_API_URL.into())]
    api_url: String,

    /// Logging verbosity. Allowed values are 'trace', 'debug', 'info', 'warn', and 'error'
    /// (case insensitive)
    #[arg(long, default_value_t = DEFAULT_LOG_LEVEL)]
    log_level: Level,

    /// Fetch weather forecasts from the Weather.gov API at this interval, in seconds
    #[arg(long, default_value_t = DEFAULT_REFERSH_SECS)]
    refresh_secs: u64,

    /// Timeout for fetching weather forecasts from the Weather.gov API, in milliseconds
    #[arg(long, default_value_t = DEFAULT_TIMEOUT_MILLIS)]
    timeout_millis: u64,

    /// Address to bind to. By default, nws_exporter will bind to public address since
    /// the purpose is to expose metrics to an external system (Prometheus or another
    /// agent for ingestion)
    #[arg(long, default_value_t = DEFAULT_BIND_ADDR.into())]
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

    let client = NwsClient::new(http_client, &opts.api_url).unwrap_or_else(|e| {
        tracing::error!(message = "unable to initialize NWS client", error = %e);
        process::exit(1)
    });

    let mut registry = <Registry>::default();
    let metrics = ForecastMetrics::new(&mut registry);
    let update = UpdateTask::new(opts.station, metrics, client, Duration::from_secs(opts.refresh_secs));

    // Make an initial request to fetch station information. This allows us to verify that the
    // station the user provided is valid and the API is available before starting the HTTP server
    // and running indefinitely.
    if let Err(e) = update.initialize().await {
        tracing::error!(message = "failed to fetch initial station information", error = %e);
        process::exit(1);
    }

    tokio::spawn(update.run());

    let state = Arc::new(RequestState { registry });
    let app = Router::new()
        .route("/metrics", get(nws_exporter::http::text_metrics_handler))
        .layer(TraceLayer::new_for_http())
        .with_state(state.clone());

    let server = axum::Server::try_bind(&opts.bind)
        .map(|s| {
            s.serve(app.into_make_service()).with_graceful_shutdown(async {
                // Wait for either SIGTERM or SIGINT to shutdown
                tokio::select! {
                    _ = sigterm() => {}
                    _ = sigint() => {}
                }
            })
        })
        .unwrap_or_else(|e| {
            tracing::error!(message = "error starting server", address = %opts.bind, err = %e);
            process::exit(1)
        });

    tracing::info!(message = "starting server", address = %opts.bind);
    server.await.unwrap();

    tracing::info!("server shutdown");
    Ok(())
}

async fn sigint() -> io::Result<()> {
    tokio::signal::ctrl_c().await
}

#[cfg(unix)]
async fn sigterm() -> io::Result<()> {
    use tokio::signal::unix::{self, SignalKind};
    unix::signal(SignalKind::terminate())?.recv().await;
    Ok(())
}

#[cfg(not(unix))]
async fn sigterm() -> io::Result<()> {
    // No SIGTERM on windows. Create a no-op future that never resolves so we can
    // have both sigterm() and sigint() above to trigger shutdown of the server.
    std::future::pending::<io::Result<()>>().await
}

/// Task for periodically updating forecast metrics for multiple stations
///
/// Perform one-time initialization of station metadata metrics and periodically
/// update the forecast metrics for a list of stations until this exporter is
/// stopped.
struct UpdateTask {
    stations: Vec<String>,
    metrics: ForecastMetrics,
    client: NwsClient,
    interval: Duration,
}

impl UpdateTask {
    fn new(stations: Vec<String>, metrics: ForecastMetrics, client: NwsClient, interval: Duration) -> Self {
        Self {
            stations,
            metrics,
            client,
            interval,
        }
    }

    /// Set station metadata metrics or return an error if station metadata could not be fetched
    async fn initialize(&self) -> Result<(), ClientError> {
        for id in self.stations.iter() {
            let station = self
                .client
                .station(id)
                .instrument(tracing::span!(Level::DEBUG, "nws_station"))
                .await?;
            self.metrics.station(&station);
        }

        Ok(())
    }

    /// Update station forecast metrics for all stations in a loop forever, logging any errors
    async fn run(self) -> ! {
        let mut interval = tokio::time::interval(self.interval);

        loop {
            let _ = interval.tick().await;
            for id in self.stations.iter() {
                match self
                    .client
                    .observation(id)
                    .instrument(tracing::span!(Level::DEBUG, "nws_observation"))
                    .await
                {
                    Ok(obs) => {
                        self.metrics.observation(&obs);
                        tracing::info!(message = "fetched new forecast", station_id = %id, observation = %obs.id);
                    }
                    Err(e) => {
                        tracing::error!(message = "failed to fetch forecast", station_id = %id, error = %e);
                    }
                }
            }
        }
    }
}
