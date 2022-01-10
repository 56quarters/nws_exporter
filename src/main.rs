use reqwest::header::{ACCEPT, USER_AGENT};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const GMAN_USER_AGENT: &str = "Gman Prometheus Exporter (https://github.com/56quarters/gman)";
const JSON_RESPONSE_TYPE: &str = "application/geo+json";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let client = Client::new();
    let res = client
        .get("https://api.weather.gov/stations/KBOS/observations/latest")
        .header(USER_AGENT, GMAN_USER_AGENT)
        .header(ACCEPT, JSON_RESPONSE_TYPE)
        .send()
        .await?
        .json::<Response>()
        //.text()
        .await?;

    println!("{:?}", res);
    //println!("{}", res);
    Ok(())
}

#[derive(Serialize, Deserialize, Debug)]
struct Response {
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
    coordinates: Vec<f64>,
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
