# Changelog

## v0.5.1 - 2023-10-21

* Dependency updates. #23
* Remove dependency on openssl. #22

## v0.5.0 - 2023-10-15

* Switch to Axum web framework. #19
* Build Docker images and binaries for each release. #20

## v0.4.0 - 2022-03-13

* Change station IDs to be a required argument (previously specified using `--station`) and
  add support for specifying multiple station IDs to collect metrics for.
* Add [Grafana dashboard](ext/dashboard.json) for visualizing metrics.

## v0.3.0 - 2022-02-05

* Emit station metadata as labels for the `nws_station` metric. #8
* Documentation improvements. #7 #9

## v0.2.0 - 2022-02-04

* Documentation.

## v0.1.0 - 2022-02-01

* Initial release.
