use clap::Parser;
use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use reqwest::header::{ACCEPT, USER_AGENT};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::net::SocketAddr;
use tracing::Level;

const UNIT_METERS: &str = "wmoUnit:m";
const UNIT_DEGREES_C: &str = "wmoUnit:degC";
const UNIT_PERCENT: &str = "wmoUnit:percent";
const UNIT_DEGREES_ANGLE: &str = "wmoUnit:degree_(angle)";
const UNIT_KPH: &str = "wmoUnit:km_h-1";
const UNIT_PASCALS: &str = "wmoUnit:Pa";

const DEFAULT_LOG_LEVEL: Level = Level::INFO;
const DEFAULT_BIND_ADDR: ([u8; 4], u16) = ([0, 0, 0, 0], 9782);

#[derive(Debug, Parser)]
#[clap(name = "gman", version = clap::crate_version ! ())]
struct GmanApplication {
    /// NWS weather station ID to fetch forecasts for
    #[clap(long)]
    station: String,

    /// Logging verbosity. Allowed values are 'trace', 'debug', 'info', 'warn', and 'error'
    /// (case insensitive)
    #[clap(long, default_value_t = DEFAULT_LOG_LEVEL)]
    log_level: Level,

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

    let client = WeatherGovClient::new(Client::new(), "https://api.weather.gov/");
    println!("{:?}", client.observation(&opts.station).await);

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
            base_url: Url::parse(base_url).unwrap(),
        }
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

        // handle non-200 here

        let obs = res.json::<Observation>().await;

        // handle malformed JSON

        Ok(obs.unwrap())
    }

    fn url(&self, station: &str) -> Url {
        let encoded_station = utf8_percent_encode(station, NON_ALPHANUMERIC);
        let mut url = self.base_url.clone();
        {
            let mut path = url.path_segments_mut().unwrap();
            path.clear()
                .push("stations")
                .push(&encoded_station.to_string())
                .push("observations")
                .push("latest");
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
