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

use crate::client::{Measurement, Observation};
use prometheus::{GaugeVec, Opts, Registry};

const NAMESPACE: &str = "gman";
const LABEL_STATION: &str = "station";
const LABEL_ELEVATION: &str = "elevation";

pub struct ForecastMetrics {
    station_info: GaugeVec,
    temperature: GaugeVec,
    dewpoint: GaugeVec,
    barometric_pressure: GaugeVec,
    visibility: GaugeVec,
    relative_humidity: GaugeVec,
    wind_chill: GaugeVec,
}

impl ForecastMetrics {
    pub fn new(reg: &Registry) -> Self {
        let station_info = GaugeVec::new(
            Opts::new("station_info", "Information about the weather station").namespace(NAMESPACE),
            &[LABEL_STATION, LABEL_ELEVATION],
        )
        .unwrap();

        let temperature = GaugeVec::new(
            Opts::new("temperature_degrees", "Temperature in celsius").namespace(NAMESPACE),
            &[LABEL_STATION],
        )
        .unwrap();

        let dewpoint = GaugeVec::new(
            Opts::new("dewpoint_degrees", "Dewpoint in celsius").namespace(NAMESPACE),
            &[LABEL_STATION],
        )
        .unwrap();

        let barometric_pressure = GaugeVec::new(
            Opts::new("barometric_pressure_pascals", "Barometric pressure in pascals").namespace(NAMESPACE),
            &[LABEL_STATION],
        )
        .unwrap();

        let visibility = GaugeVec::new(
            Opts::new("visibility_meters", "Visibility in meters").namespace(NAMESPACE),
            &[LABEL_STATION],
        )
        .unwrap();

        let relative_humidity = GaugeVec::new(
            Opts::new("relative_humidity", "Relative humidity (0-100)").namespace(NAMESPACE),
            &[LABEL_STATION],
        )
        .unwrap();

        let wind_chill = GaugeVec::new(
            Opts::new("wind_chill_degrees", "Temperature with wind chill in celsius").namespace(NAMESPACE),
            &[LABEL_STATION],
        )
        .unwrap();

        reg.register(Box::new(station_info.clone())).unwrap();
        reg.register(Box::new(temperature.clone())).unwrap();
        reg.register(Box::new(dewpoint.clone())).unwrap();
        reg.register(Box::new(barometric_pressure.clone())).unwrap();
        reg.register(Box::new(visibility.clone())).unwrap();
        reg.register(Box::new(relative_humidity.clone())).unwrap();
        reg.register(Box::new(wind_chill.clone())).unwrap();

        Self {
            station_info,
            temperature,
            dewpoint,
            barometric_pressure,
            visibility,
            relative_humidity,
            wind_chill,
        }
    }

    pub fn observe(&self, obs: &Observation) {
        self.set_station_info(obs);

        let station = &obs.properties.station;
        self.temperature
            .with_label_values(&[station])
            .set(self.must_measurement(&obs.properties.temperature));

        self.dewpoint
            .with_label_values(&[station])
            .set(self.must_measurement(&obs.properties.dewpoint));

        self.barometric_pressure
            .with_label_values(&[station])
            .set(self.must_measurement(&obs.properties.barometric_pressure));

        self.visibility
            .with_label_values(&[station])
            .set(self.must_measurement(&obs.properties.visibility));

        self.relative_humidity
            .with_label_values(&[station])
            .set(self.must_measurement(&obs.properties.relative_humidity));

        self.wind_chill
            .with_label_values(&[station])
            .set(self.must_measurement(&obs.properties.wind_chill));
    }

    fn set_station_info(&self, obs: &Observation) {
        self.station_info
            .with_label_values(&[
                &obs.properties.station,
                &format!("{}", self.must_measurement(&obs.properties.elevation)),
            ])
            .set(1.0);
    }

    fn must_measurement(&self, measurement: &Measurement) -> f64 {
        measurement.value.unwrap()
    }
}
