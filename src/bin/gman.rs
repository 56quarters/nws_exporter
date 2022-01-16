// Gman - Prometheus metrics exporter for api.weather.gov
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
use gman::client::WeatherGovClient;
use gman::http::{http_route, RequestContext};
use gman::metrics::ForecastMetrics;
use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
use reqwest::Client;
use std::error::Error;
use std::io;
use std::net::SocketAddr;
use std::process;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::signal::unix::{self, SignalKind};
use tracing::{event, span, Instrument, Level};

const DEFAULT_LOG_LEVEL: Level = Level::INFO;
const DEFAULT_BIND_ADDR: ([u8; 4], u16) = ([0, 0, 0, 0], 9782);
const DEFAULT_REFERSH_SECS: u64 = 300;
const DEFAULT_API_URL: &str = "https://api.weather.gov/";

#[derive(Debug, Parser)]
#[clap(name = "gman", version = clap::crate_version!())]
struct GmanApplication {
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

    /// Address to bind to. By default, gman will bind to public address since
    /// the purpose is to expose metrics to an external system (Prometheus or another
    /// agent for ingestion)
    #[clap(long, default_value_t = DEFAULT_BIND_ADDR.into())]
    bind: SocketAddr,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error + Send + Sync>> {
    let opts = GmanApplication::parse();
    tracing::subscriber::set_global_default(
        tracing_subscriber::FmtSubscriber::builder()
            .with_max_level(opts.log_level)
            .finish(),
    )
    .expect("failed to set tracing subscriber");

    let startup = Instant::now();
    let registry = prometheus::default_registry().clone();
    let metrics = ForecastMetrics::new(&registry);
    let context = Arc::new(RequestContext::new(registry));
    let service = make_service_fn(move |_| {
        let context = context.clone();

        async move {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                http_route(req, context.clone()).instrument(span!(Level::DEBUG, "gman_request"))
            }))
        }
    });

    let server = Server::try_bind(&opts.bind).unwrap_or_else(|e| {
        event!(
            Level::ERROR,
            message = "server failed to start",
            error = %e,
            address = %opts.bind,
            api_url = %opts.api_url,
        );

        process::exit(1);
    });

    // TODO(56quarters): Do a client.station() call to make sure the station supplied by the
    //  user is valid before going into a loop making requests for it.
    let client = WeatherGovClient::new(Client::new(), &opts.api_url);
    let station = opts.station.clone();
    let interval = Duration::from_secs(opts.refresh_secs);

    tokio::spawn(async move {
        let mut interval_stream = tokio::time::interval(interval);

        loop {
            let _ = interval_stream.tick().await;

            match client
                .observation(&station)
                .instrument(span!(Level::DEBUG, "gman_observation"))
                .await
            {
                Ok(obs) => {
                    metrics.observe(&obs);
                    event!(
                        Level::DEBUG,
                        message = "fetched new forecast",
                        observation = %obs.id,
                        runtime_secs = startup.elapsed().as_secs(),
                    );
                }
                Err(e) => {
                    event!(
                        Level::ERROR,
                        message = "failed to fetch forecast",
                        error = %e,
                        runtime_secs = startup.elapsed().as_secs(),
                    );
                }
            }
        }
    });

    event!(
        Level::INFO,
        message = "server started",
        address = %opts.bind,
        api_url = %opts.api_url,
    );

    server
        .serve(service)
        .with_graceful_shutdown(async {
            // Wait for either SIGTERM or SIGINT to shutdown
            tokio::select! {
                _ = sigterm() => {}
                _ = sigint() => {}
            }
        })
        .await?;

    event!(
        Level::INFO,
        message = "server shutdown",
        runtime_secs = %startup.elapsed().as_secs(),
    );

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
