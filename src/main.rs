use clap::Parser;
use hyper::header::CONTENT_TYPE;
use hyper::service::{make_service_fn, service_fn};
use hyper::Server;
use hyper::{Body, Method, Request, Response, StatusCode};
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use prometheus::{Encoder, TextEncoder, TEXT_FORMAT};
use reqwest::header::{ACCEPT, USER_AGENT};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::io;
use std::net::SocketAddr;
use std::process;
use std::time::Duration;
use tokio::signal::unix::{self, SignalKind};
use tracing::Level;

const UNIT_METERS: &str = "wmoUnit:m";
const UNIT_DEGREES_C: &str = "wmoUnit:degC";
const UNIT_PERCENT: &str = "wmoUnit:percent";
const UNIT_DEGREES_ANGLE: &str = "wmoUnit:degree_(angle)";
const UNIT_KPH: &str = "wmoUnit:km_h-1";
const UNIT_PASCALS: &str = "wmoUnit:Pa";

const DEFAULT_LOG_LEVEL: Level = Level::INFO;
const DEFAULT_BIND_ADDR: ([u8; 4], u16) = ([0, 0, 0, 0], 9782);
const DEFAULT_REFERSH_SECS: u64 = 300;
const DEFAULT_API_URL: &'static str = "https://api.weather.gov/";

#[derive(Debug, Parser)]
#[clap(name = "gman", version = clap::crate_version ! ())]
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

    // TODO(56quarters): Put a registry into a request context instead of using the global one?
    let service = make_service_fn(move |_| async move { Ok::<_, hyper::Error>(service_fn(http_route)) });
    let server = Server::try_bind(&opts.bind).unwrap_or_else(|e| {
        // TODO(56quarters): Error logging
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
            // TODO(56quarters): Something that owns a bunch of metrics and updates them
            //  based on the results of the API call.

            let _ = interval_stream.tick().await;

            // TODO(56quarters): Handle errors here and log them
            println!("{:?}", client.observation(&station).await);
        }
    });

    // TODO(56quarters): info logging

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

    // TODO(56quarters): info logging

    Ok(())
}

struct WeatherGovClient {
    client: Client,
    base_url: Url,
}

impl WeatherGovClient {
    const USER_AGENT: &'static str = "Gman Prometheus Exporter (https://github.com/56quarters/gman)";
    const JSON_RESPONSE: &'static str = "application/geo+json";

    fn new(client: Client, base_url: &str) -> Self {
        WeatherGovClient {
            client,
            // TODO(56quarters): Handle this better
            base_url: Url::parse(base_url).unwrap(),
        }
    }

    async fn station(&self, station: &str) -> Result<(), reqwest::Error> {
        todo!("method to show some information about a weather station, run once at startup (validation)")
    }

    async fn observation(&self, station: &str) -> Result<Observation, reqwest::Error> {
        let request_url = self.url(station);
        println!("URL: {}", request_url);

        let res = self
            .client
            .get(request_url)
            .header(USER_AGENT, Self::USER_AGENT)
            .header(ACCEPT, Self::JSON_RESPONSE)
            .send()
            .await?;

        // TODO(56quarters): handle non-200 here

        let obs = res.json::<Observation>().await;

        // TODO(56quarters): handle malformed JSON

        Ok(obs.unwrap())
    }

    fn url(&self, station: &str) -> Url {
        let encoded_station = utf8_percent_encode(station, NON_ALPHANUMERIC);
        let mut url = self.base_url.clone();
        {
            // TODO(56quarters): Should this be a panic?
            url.path_segments_mut()
                .map(|mut p| {
                    p.clear()
                        .push("stations")
                        .push(&encoded_station.to_string())
                        .push("observations")
                        .push("latest");
                })
                .expect("unable to modify URL path segments");
        }
        url
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct Observation {
    #[serde(alias = "id")]
    id: String,
    #[serde(alias = "type")]
    type_: String,
    #[serde(alias = "geometry")]
    geometry: Geometry,
    #[serde(alias = "properties")]
    properties: Properties,
}

#[derive(Serialize, Deserialize, Debug)]
struct Geometry {
    #[serde(alias = "type")]
    type_: String,
    #[serde(alias = "coordinates")]
    coordinates: [f64; 2],
}

#[derive(Serialize, Deserialize, Debug)]
struct Properties {
    #[serde(alias = "@id")]
    id: String,
    #[serde(alias = "@type")]
    type_: String,
    #[serde(alias = "elevation")]
    elevation: Measurement,
    #[serde(alias = "station")]
    station: String,
    #[serde(alias = "timestamp")]
    timestamp: String,
    #[serde(alias = "rawMessage")]
    raw_message: String,
    #[serde(alias = "textDescription")]
    description: String,
    #[serde(alias = "icon")]
    icon: String,
    #[serde(alias = "presentWeather")]
    present_weather: Vec<Weather>,
    #[serde(alias = "temperature")]
    temperature: Measurement,
    #[serde(alias = "dewpoint")]
    dewpoint: Measurement,
    #[serde(alias = "windDirection")]
    wind_direction: Measurement,
    #[serde(alias = "windSpeed")]
    wind_speed: Measurement,
    #[serde(alias = "windGust")]
    wind_gust: Measurement,
    #[serde(alias = "barometricPressure")]
    barometric_pressure: Measurement,
    #[serde(alias = "seaLevelPressure")]
    sea_level_pressure: Measurement,
    #[serde(alias = "visibility")]
    visibility: Measurement,
    #[serde(alias = "maxTemperatureLast24Hours")]
    max_temperature_last_24_hours: Measurement,
    #[serde(alias = "minTemperatureLast24Hours")]
    min_temperature_last_24_hours: Measurement,
    #[serde(alias = "precipitationLastHour")]
    precipitation_last_hour: Measurement,
    #[serde(alias = "precipitationLast3Hours")]
    precipitation_last_3_hours: Measurement,
    #[serde(alias = "precipitationLast6Hours")]
    precipitation_last_6_hours: Measurement,
    #[serde(alias = "relativeHumidity")]
    relative_humidity: Measurement,
    #[serde(alias = "windChill")]
    wind_chill: Measurement,
    #[serde(alias = "heatIndex")]
    heat_index: Measurement,
    #[serde(alias = "cloudLayers")]
    cloud_layers: Vec<CloudLayer>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Weather {
    #[serde(alias = "intensity")]
    intensity: String,
    #[serde(alias = "modifier")]
    modifier: Option<String>,
    #[serde(alias = "weather")]
    weather: String,
    #[serde(alias = "rawString")]
    raw_string: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct CloudLayer {
    #[serde(alias = "base")]
    base: Measurement,
    #[serde(alias = "amount")]
    amount: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct Measurement {
    #[serde(alias = "unitCode")]
    unit_code: String,
    #[serde(alias = "value")]
    value: Option<f64>,
    #[serde(alias = "qualityControl")]
    quality_control: Option<String>,
}

async fn http_route(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_owned();

    let res = match (&method, path.as_ref()) {
        (&Method::GET, "/metrics") => {
            let mut buf = Vec::new();
            let encoder = TextEncoder::new();

            match encoder.encode(&prometheus::gather(), &mut buf) {
                Ok(_) => Response::builder()
                    .status(StatusCode::OK)
                    .header(CONTENT_TYPE, TEXT_FORMAT)
                    .body(Body::from(buf))
                    .unwrap(),

                // TODO(56quarters): Error logging
                Err(e) => http_status_no_body(StatusCode::SERVICE_UNAVAILABLE),
            }
        }

        (_, "/metrics") => http_status_no_body(StatusCode::METHOD_NOT_ALLOWED),

        _ => http_status_no_body(StatusCode::NOT_FOUND),
    };

    Ok(res)
}

fn http_status_no_body(code: StatusCode) -> Response<Body> {
    Response::builder().status(code).body(Body::empty()).unwrap()
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
