# nws_exporter

![build status](https://github.com/56quarters/nws_exporter/actions/workflows/rust.yml/badge.svg)
[![docs.rs](https://docs.rs/nws_exporter/badge.svg)](https://docs.rs/nws_exporter/)
[![crates.io](https://img.shields.io/crates/v/nws_exporter.svg)](https://crates.io/crates/nws_exporter/)

Prometheus metrics exporter for api.weather.gov

## Features

`nws_exporter` fetches weather information for a particular [NWS station] using the [api.weather.gov] API and emits
it as Prometheus metrics. Users must pick a particular station to fetch weather information from. The following
metrics are emitted when available (not all fields are available for all stations).

* `nws_station{station=$STATION, station_id=$STATION_ID, station_name=$STATION_NAME}` - Station metadata
* `nws_elevation_meters{station=$STATION}` - Elevation of the station, in meters.
* `nws_temperature_degrees{station=$STATION}` - Temperature, in degrees celsius.
* `nws_dewpoint_degrees{station=$STATION}` - Dewpoint, in degrees celsius.
* `nws_barometric_pressure_pascals{station=$STATION}` - Barometric pressure, in pascals.
* `nws_visibility_meters{station=$STATION}` - Visibility, in meters.
* `nws_relative_humidity{station=$STATION}` - Relative humidity (0-100).
* `nws_wind_chill_degrees{station=$STATION}` - Temperature with wind chill, in degrees celsius.

[NWS station]: https://www.weather.gov/documentation/services-web-api#/default/obs_stations
[api.weather.gov]: https://www.weather.gov/documentation/services-web-api

## Install

There are multiple ways to install `nws_exporter` listed below.

### Binaries

Binaries are published for GNU/Linux (x86_64), Windows (x86_64), and MacOS (x86_64 and aarch64)
for [each release](https://github.com/56quarters/nws_exporter/releases).

### Docker

Docker images for GNU/Linux (amd64) are published for [each release](https://hub.docker.com/r/56quarters/nws_exporter).

### Cargo

`nws_exporter` along with its dependencies can be downloaded and built from source using the
Rust `cargo` tool. Note that this requires you have a Rust toolchain installed.

To install:

```
cargo install nws_exporter
```

To uninstall:

```
cargo uninstall nws_exporter
```

### Source

`nws_exporter` along with its dependencies can be built from the latest sources on Github using
the Rust `cargo` tool. Note that this requires you have Git and a Rust toolchain installed.

Get the sources:

```
git clone https://github.com/56quarters/nws_exporter.git && cd nws_exporter
```

Install from local sources:

```
cargo install --path .
```

To uninstall:

```
cargo uninstall nws_exporter
```

## Usage

### Picking a station

In order to export NWS forecast information, `nws_exporter` needs to be told which NWS station to request
information for. You can get a list of the available stations in your state by using the API itself. An
example of this using `curl` is below.

```text
curl -sS 'https://api.weather.gov/stations?state=MA' | jq | less
```

This command lists all available stations in the state of Massachusetts. The `properties.stationIdentifier`
field for each station is the ID that you should use with `nws_exporter`. For example `KBOS` is the ID for
the station at Logan Airport in Boston.

You can then run `nws_exporter` for this station as demonstrated below.

```text
./nws_exporter KBOS
```

### Run

You can run `nws_exporter` as a Systemd service using the [provided unit file](ext/nws_exporter.service). This
unit file  assumes that you have copied the resulting `nws_exporter` binary to `/usr/local/bin/nws_exporter`.
Make sure to edit the unit file to use a station near you that you picked in the previous step.

```text
sudo cp target/release/nws_exporter /usr/local/bin/nws_exporter
sudo cp ext/nws_exporter.service /etc/systemd/system/nws_exporter.service
sudo sed -i 's/KBOS/YOUR_STATION/' /etc/systemd/system/nws_exporter.service
sudo systemctl daemon-reload
sudo systemctl enable nws_exporter.service
sudo systemctl start nws_exporter.serivce
```

### Prometheus

Prometheus metrics are exposed on port `9782` at `/metrics`. Once `nws_exporter`
is running, configure scrapes of it by your Prometheus server. Add the host running
`nws_exporter` as a target under the Prometheus `scrape_configs` section as described by
the example below.

```yaml
# Sample config for Prometheus.

global:
  scrape_interval:     15s
  evaluation_interval: 15s
  external_labels:
    monitor: 'my_prom'

scrape_configs:
- job_name: nws_exporter
  static_configs:
  - targets: ['example:9782']
```

## License

nws_exporter is available under the terms of the [GPL, version 3](LICENSE).

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be licensed as above, without any
additional terms or conditions.
