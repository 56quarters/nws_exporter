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
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::family::Family;
use prometheus_client::metrics::gauge::Gauge;
use prometheus_client::registry::Registry;
use std::sync::atomic::AtomicU64;

#[derive(Debug, Clone, Hash, PartialEq, Eq, EncodeLabelSet)]
struct Labels {
    station: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, EncodeLabelSet)]
struct InfoLabels {
    station: String,
    station_id: String,
    station_name: String,
}

/// Holder for metrics that can be set from an `Observation` response.
///
/// All metrics are created and registered upon call to `ForecastMetrics::new()`. Metrics
/// all share the prefix "nws_" and have a "station" label that will be set to the full
/// ID of the station (e.g. `{station="https://api.weather.gov/stations/KBOS"}`)
pub struct ForecastMetrics {
    station: Family<InfoLabels, Gauge<f64, AtomicU64>>,
    elevation: Family<Labels, Gauge<f64, AtomicU64>>,
    temperature: Family<Labels, Gauge<f64, AtomicU64>>,
    dewpoint: Family<Labels, Gauge<f64, AtomicU64>>,
    barometric_pressure: Family<Labels, Gauge<f64, AtomicU64>>,
    visibility: Family<Labels, Gauge<f64, AtomicU64>>,
    relative_humidity: Family<Labels, Gauge<f64, AtomicU64>>,
    wind_chill: Family<Labels, Gauge<f64, AtomicU64>>,
}

impl ForecastMetrics {
    /// Create a new `ForecastMetrics` and register each metric with the provided `Registry`.
    pub fn new(reg: &mut Registry) -> Self {
        let station = Family::<InfoLabels, Gauge<f64, AtomicU64>>::default();
        let elevation = Family::<Labels, Gauge<f64, AtomicU64>>::default();
        let temperature = Family::<Labels, Gauge<f64, AtomicU64>>::default();
        let dewpoint = Family::<Labels, Gauge<f64, AtomicU64>>::default();
        let barometric_pressure = Family::<Labels, Gauge<f64, AtomicU64>>::default();
        let visibility = Family::<Labels, Gauge<f64, AtomicU64>>::default();
        let relative_humidity = Family::<Labels, Gauge<f64, AtomicU64>>::default();
        let wind_chill = Family::<Labels, Gauge<f64, AtomicU64>>::default();

        reg.register("nws_station", "Station metadata", station.clone());
        reg.register("nws_elevation_meters", "Elevation in meters", elevation.clone());
        reg.register("nws_temperature_degrees", "Temperature in celsius", temperature.clone());
        reg.register("nws_dewpoint_degrees", "Dewpoint in celsius", dewpoint.clone());
        reg.register(
            "nws_barometric_pressure_pascals",
            "Barometric pressure in pascals",
            barometric_pressure.clone(),
        );
        reg.register("nws_visibility_meters", "Visibility in meters", visibility.clone());
        reg.register(
            "nws_relative_humidity",
            "Relative humidity (0-100)",
            relative_humidity.clone(),
        );
        reg.register(
            "nws_wind_chill_degrees",
            "Temperature with wind chill in celsius",
            wind_chill.clone(),
        );

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
        let labels = InfoLabels {
            station: station.properties.id.clone(),
            station_id: station.properties.station_identifier.clone(),
            station_name: station.properties.name.clone(),
        };

        self.station.get_or_create(&labels).set(1.0);
    }

    /// Set metrics from the provided forecast if the relevant value exists.
    ///
    /// If the forecast doesn't contain a value for a particular metric, the metric will
    /// not be updated.
    pub fn observation(&self, obs: &Observation) {
        let labels = Labels {
            station: obs.properties.station.clone(),
        };
        self.set_from_measurement(&labels, &self.elevation, &obs.properties.elevation);
        self.set_from_measurement(&labels, &self.temperature, &obs.properties.temperature);
        self.set_from_measurement(&labels, &self.dewpoint, &obs.properties.dewpoint);
        self.set_from_measurement(&labels, &self.barometric_pressure, &obs.properties.barometric_pressure);
        self.set_from_measurement(&labels, &self.visibility, &obs.properties.visibility);
        self.set_from_measurement(&labels, &self.relative_humidity, &obs.properties.relative_humidity);
        self.set_from_measurement(&labels, &self.wind_chill, &obs.properties.wind_chill);
    }

    fn set_from_measurement(
        &self,
        labels: &Labels,
        gauge: &Family<Labels, Gauge<f64, AtomicU64>>,
        measurement: &Measurement,
    ) {
        if let Some(v) = measurement.value {
            gauge.get_or_create(labels).set(v);
        }
    }
}
