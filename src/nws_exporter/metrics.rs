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

use crate::client::{Measurement, Observation, Station};
use prometheus::{GaugeVec, Opts, Registry};

const LABEL_STATION: &str = "station";
const LABEL_STATION_ID: &str = "station_id";
const LABEL_STATION_NAME: &str = "station_name";

/// Holder for metrics that can be set from an `Observation` response.
///
/// All metrics are created and registered upon call to `ForecastMetrics::new()`. Metrics
/// all share the prefix "nws_" and have a "station" label that will be set to the full
/// ID of the station (e.g. `{station="https://api.weather.gov/stations/KBOS"}`)
#[derive(Debug)]
pub struct ForecastMetrics {
    station: GaugeVec,
    elevation: GaugeVec,
    temperature: GaugeVec,
    dewpoint: GaugeVec,
    barometric_pressure: GaugeVec,
    visibility: GaugeVec,
    relative_humidity: GaugeVec,
    wind_chill: GaugeVec,
}

impl ForecastMetrics {
    /// Create a new `ForecastMetrics` and register each metric with the provided `Registry`.
    ///
    /// # Panics
    ///
    /// If any metric cannot be created or registered, this method will panic.
    pub fn new(reg: &Registry) -> Self {
        let station = GaugeVec::new(
            Opts::new("nws_station", "Station metadata"),
            &[LABEL_STATION, LABEL_STATION_ID, LABEL_STATION_NAME],
        )
        .unwrap();
        let elevation = GaugeVec::new(
            Opts::new("nws_elevation_meters", "Elevation in meters"),
            &[LABEL_STATION],
        )
        .unwrap();
        let temperature = GaugeVec::new(
            Opts::new("nws_temperature_degrees", "Temperature in celsius"),
            &[LABEL_STATION],
        )
        .unwrap();
        let dewpoint = GaugeVec::new(
            Opts::new("nws_dewpoint_degrees", "Dewpoint in celsius"),
            &[LABEL_STATION],
        )
        .unwrap();
        let barometric_pressure = GaugeVec::new(
            Opts::new("nws_barometric_pressure_pascals", "Barometric pressure in pascals"),
            &[LABEL_STATION],
        )
        .unwrap();
        let visibility = GaugeVec::new(
            Opts::new("nws_visibility_meters", "Visibility in meters"),
            &[LABEL_STATION],
        )
        .unwrap();
        let relative_humidity = GaugeVec::new(
            Opts::new("nws_relative_humidity", "Relative humidity (0-100)"),
            &[LABEL_STATION],
        )
        .unwrap();
        let wind_chill = GaugeVec::new(
            Opts::new("nws_wind_chill_degrees", "Temperature with wind chill in celsius"),
            &[LABEL_STATION],
        )
        .unwrap();

        reg.register(Box::new(station.clone())).unwrap();
        reg.register(Box::new(elevation.clone())).unwrap();
        reg.register(Box::new(temperature.clone())).unwrap();
        reg.register(Box::new(dewpoint.clone())).unwrap();
        reg.register(Box::new(barometric_pressure.clone())).unwrap();
        reg.register(Box::new(visibility.clone())).unwrap();
        reg.register(Box::new(relative_humidity.clone())).unwrap();
        reg.register(Box::new(wind_chill.clone())).unwrap();

        Self {
            station,
            elevation,
            temperature,
            dewpoint,
            barometric_pressure,
            visibility,
            relative_humidity,
            wind_chill,
        }
    }

    /// Set station metadata as labels on a single gauge with values from the provided station
    pub fn station(&self, station: &Station) {
        self.station
            .with_label_values(&[
                &station.properties.id,
                &station.properties.station_identifier,
                &station.properties.name,
            ])
            .set(1.0);
    }

    /// Set metrics from the provided forecast if the relevant value exists.
    ///
    /// If the forecast doesn't contain a value for a particular metric, the metric will
    /// not be updated.
    pub fn observation(&self, obs: &Observation) {
        let station = &obs.properties.station;
        self.set_from_measurement(station, &self.elevation, &obs.properties.elevation);
        self.set_from_measurement(station, &self.temperature, &obs.properties.temperature);
        self.set_from_measurement(station, &self.dewpoint, &obs.properties.dewpoint);
        self.set_from_measurement(station, &self.barometric_pressure, &obs.properties.barometric_pressure);
        self.set_from_measurement(station, &self.visibility, &obs.properties.visibility);
        self.set_from_measurement(station, &self.relative_humidity, &obs.properties.relative_humidity);
        self.set_from_measurement(station, &self.wind_chill, &obs.properties.wind_chill);
    }

    fn set_from_measurement(&self, station: &str, gauge: &GaugeVec, measurement: &Measurement) {
        if let Some(v) = measurement.value {
            gauge.with_label_values(&[station]).set(v);
        }
    }
}
