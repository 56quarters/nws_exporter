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

use percent_encoding::{utf8_percent_encode, NON_ALPHANUMERIC};
use reqwest::header::{ACCEPT, USER_AGENT};
use reqwest::{Client, Response, StatusCode, Url};
use serde::{Deserialize, Serialize};
use std::error;
use std::fmt;

#[derive(Debug)]
pub enum ClientError {
    Internal(reqwest::Error),
    InvalidStation(String),
    Unexpected(StatusCode, Url),
}

impl fmt::Display for ClientError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Internal(e) => write!(f, "{}", e),
            Self::InvalidStation(s) => write!(f, "invalid station {}", s),
            Self::Unexpected(status, url) => write!(f, "unexpected status {} for {}", status, url),
        }
    }
}

impl error::Error for ClientError {
    fn source(&self) -> Option<&(dyn error::Error + 'static)> {
        match self {
            Self::Internal(e) => Some(e),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct WeatherGovClient {
    client: Client,
    base_url: Url,
}

impl WeatherGovClient {
    const USER_AGENT: &'static str = "Gman Prometheus Exporter (https://github.com/56quarters/nws_exporter)";
    const JSON_RESPONSE: &'static str = "application/geo+json";

    pub fn new(client: Client, base_url: &str) -> Self {
        WeatherGovClient {
            client,
            // TODO(56quarters): Handle this better
            base_url: Url::parse(base_url).unwrap(),
        }
    }

    pub async fn station(&self, station: &str) -> Result<Station, ClientError> {
        let station_url = self.station_url(station);
        tracing::debug!(message = "making station information request", url = %station_url);

        let res = self.make_request(station, station_url).await?;
        Ok(res.json::<Station>().await.map_err(ClientError::Internal)?)
    }

    pub async fn observation(&self, station: &str) -> Result<Observation, ClientError> {
        let request_url = self.observation_url(station);
        tracing::debug!(message = "making latest observation request", url = %request_url);

        let res = self.make_request(station, request_url).await?;
        Ok(res.json::<Observation>().await.map_err(ClientError::Internal)?)
    }

    async fn make_request<S: Into<String>>(&self, station: S, url: Url) -> Result<Response, ClientError> {
        let res = self
            .client
            .get(url.clone())
            .header(USER_AGENT, Self::USER_AGENT)
            .header(ACCEPT, Self::JSON_RESPONSE)
            .send()
            .await
            .map_err(ClientError::Internal)?;

        let status = res.status();
        if status == StatusCode::OK {
            Ok(res)
        } else if status == StatusCode::NOT_FOUND {
            Err(ClientError::InvalidStation(station.into()))
        } else {
            Err(ClientError::Unexpected(status, url))
        }
    }

    fn station_url(&self, station: &str) -> Url {
        let encoded_station = utf8_percent_encode(station, NON_ALPHANUMERIC);
        let mut url = self.base_url.clone();
        {
            url.path_segments_mut()
                .map(|mut p| {
                    p.clear().push("stations").push(&encoded_station.to_string());
                })
                .expect("unable to modify station URL path segments");
        }

        url
    }

    fn observation_url(&self, station: &str) -> Url {
        let mut url = self.station_url(station);
        {
            url.path_segments_mut()
                .map(|mut p| {
                    p.push("observations").push("latest");
                })
                .expect("unable to modify observation URL path segments");
        }

        url
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Station {
    #[serde(alias = "id")]
    pub id: String,
    #[serde(alias = "type")]
    pub type_: String,
    #[serde(alias = "properties")]
    pub properties: StationProperties,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StationProperties {
    #[serde(alias = "@id")]
    pub id: String,
    #[serde(alias = "@type")]
    pub type_: String,
    #[serde(alias = "elevation")]
    pub elevation: Measurement,
    #[serde(alias = "stationIdentifier")]
    pub station_identifier: String,
    #[serde(alias = "name")]
    pub name: String,
    #[serde(alias = "timezone")]
    pub timezone: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Observation {
    #[serde(alias = "id")]
    pub id: String,
    #[serde(alias = "type")]
    pub type_: String,
    #[serde(alias = "properties")]
    pub properties: ObservationProperties,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ObservationProperties {
    #[serde(alias = "@id")]
    pub id: String,
    #[serde(alias = "@type")]
    pub type_: String,
    #[serde(alias = "elevation")]
    pub elevation: Measurement,
    #[serde(alias = "station")]
    pub station: String,
    #[serde(alias = "timestamp")]
    pub timestamp: String,
    #[serde(alias = "rawMessage")]
    pub raw_message: Option<String>,
    #[serde(alias = "textDescription")]
    pub description: Option<String>,
    #[serde(alias = "icon")]
    pub icon: Option<String>,
    #[serde(alias = "presentWeather")]
    pub present_weather: Vec<Weather>,
    #[serde(alias = "temperature")]
    pub temperature: Measurement,
    #[serde(alias = "dewpoint")]
    pub dewpoint: Measurement,
    #[serde(alias = "windDirection")]
    pub wind_direction: Measurement,
    #[serde(alias = "windSpeed")]
    pub wind_speed: Measurement,
    #[serde(alias = "windGust")]
    pub wind_gust: Measurement,
    #[serde(alias = "barometricPressure")]
    pub barometric_pressure: Measurement,
    #[serde(alias = "seaLevelPressure")]
    pub sea_level_pressure: Measurement,
    #[serde(alias = "visibility")]
    pub visibility: Measurement,
    #[serde(alias = "relativeHumidity")]
    pub relative_humidity: Measurement,
    #[serde(alias = "windChill")]
    pub wind_chill: Measurement,
    #[serde(alias = "heatIndex")]
    pub heat_index: Measurement,
    #[serde(alias = "cloudLayers")]
    pub cloud_layers: Vec<CloudLayer>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Weather {
    #[serde(alias = "weather")]
    pub weather: String,
    #[serde(alias = "rawString")]
    pub raw_string: String,
    #[serde(alias = "intensity")]
    pub intensity: Option<String>,
    #[serde(alias = "modifier")]
    pub modifier: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CloudLayer {
    #[serde(alias = "base")]
    pub base: Measurement,
    #[serde(alias = "amount")]
    pub amount: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Measurement {
    #[serde(alias = "unitCode")]
    pub unit_code: String,
    #[serde(alias = "value")]
    pub value: Option<f64>,
    #[serde(alias = "qualityControl")]
    pub quality_control: Option<String>,
}
