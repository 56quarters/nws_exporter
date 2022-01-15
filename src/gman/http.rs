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

use hyper::header::CONTENT_TYPE;
use hyper::{Body, Method, Request, Response, StatusCode};
use prometheus::{Encoder, TextEncoder, TEXT_FORMAT};
use tracing::{event, Level};

pub async fn http_route(req: Request<Body>) -> Result<Response<Body>, hyper::Error> {
    let method = req.method().clone();
    let path = req.uri().path().to_owned();

    let res = match (&method, path.as_ref()) {
        (&Method::GET, "/metrics") => {
            let mut buf = Vec::new();
            let encoder = TextEncoder::new();

            match encoder.encode(&prometheus::gather(), &mut buf) {
                Ok(_) => {
                    event!(
                        Level::DEBUG,
                        message = "encoded prometheus metrics to text format",
                        num_bytes = buf.len(),
                    );

                    Response::builder()
                        .status(StatusCode::OK)
                        .header(CONTENT_TYPE, TEXT_FORMAT)
                        .body(Body::from(buf))
                        .unwrap()
                }
                Err(e) => {
                    event!(
                        Level::ERROR,
                        message = "error encoding metrics",
                        error = %e,
                    );

                    http_status_no_body(StatusCode::SERVICE_UNAVAILABLE)
                }
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
