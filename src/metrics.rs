use metrics;
use metrics_exporter_prometheus;
use std::net::SocketAddr;

use super::{Error, ErrorKind};

/// metric generates a namespaced metric name where the supplied `name` is prefixed with `subbers_` making it `subbers_<name>`.
macro_rules! metric {
    ($name:literal) => {
        concat!("subbers_", $name)
    };
}

pub fn init_prometheus_endpoint(listen_addr: SocketAddr, version: String) -> Result<(), Error> {
    metrics_exporter_prometheus::PrometheusBuilder::new()
        .with_http_listener(listen_addr)
        .install()
        .map_err(|e| Error::new(ErrorKind::Internal).with_source(e))?;

    metrics::describe_gauge!(
        metric!("build_info"),
        metrics::Unit::Count,
        "application build information"
    );
    metrics::gauge!(metric!("build_info"), "version" => version).set(1);

    Ok(())
}

#[cfg(test)]
#[test]
fn test_prometheus_endpoint() {
    use std::net::Ipv4Addr;

    let listen_addr = SocketAddr::new(Ipv4Addr::LOCALHOST.into(), 48081);
    init_prometheus_endpoint(listen_addr, "test-version".to_string()).unwrap();

    let endpoint = format!("http://{}:{}/metrics", listen_addr.ip(), listen_addr.port());

    let response = reqwest::blocking::get(endpoint).unwrap();
    assert!(response.status() == reqwest::StatusCode::OK);
    assert!(
        response
            .text()
            .expect("response body must be present")
            .contains(r#"subbers_build_info{version="#)
    );
}
