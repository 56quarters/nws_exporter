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

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use reqwest::header::{ACCEPT, USER_AGENT};
use reqwest::{Client, Url};
use serde::{Deserialize, Serialize};

pub struct WeatherGovClient {
    client: Client,
    base_url: Url,
}

impl WeatherGovClient {
    const USER_AGENT: &'static str = "Gman Prometheus Exporter (https://github.com/56quarters/gman)";
    const JSON_RESPONSE: &'static str = "application/geo+json";

    pub fn new(client: Client, base_url: &str) -> Self {
        WeatherGovClient {
            client,
            // TODO(56quarters): Handle this better
            base_url: Url::parse(base_url).unwrap(),
        }
    }

    pub async fn station(&self, _station: &str) -> Result<(), reqwest::Error> {
        todo!("method to show some information about a weather station, run once at startup (validation)")
    }

    pub async fn observation(&self, station: &str) -> Result<Observation, reqwest::Error> {
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
pub struct Observation {
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
pub struct Geometry {
    #[serde(alias = "type")]
    type_: String,
    #[serde(alias = "coordinates")]
    coordinates: [f64; 2],
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Properties {
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
pub struct Weather {
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
pub struct CloudLayer {
    #[serde(alias = "base")]
    base: Measurement,
    #[serde(alias = "amount")]
    amount: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Measurement {
    #[serde(alias = "unitCode")]
    unit_code: String,
    #[serde(alias = "value")]
    value: Option<f64>,
    #[serde(alias = "qualityControl")]
    quality_control: Option<String>,
}
