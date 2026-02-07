use super::{Error, ErrorKind};

use axum::{
    Router,
    extract::State,
    http::{StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use prometheus_client::{
    metrics::{family::Family, gauge::Gauge},
    registry::Registry,
};
use std::sync::Arc;

pub fn create_registry(version: String) -> Result<Registry, Error> {
    let mut registry = Registry::with_prefix("subbers");

    // let build_info: Gauge = Gauge::default();
    let build_info = Family::<Vec<(String, String)>, Gauge>::default();
    build_info
        .get_or_create(&vec![("version".to_owned(), version)])
        .set(1);

    registry.register("build_info", "application build information", build_info);

    Ok(registry)
}

#[derive(Clone)]
struct MetricState {
    prometheus_registry: Arc<prometheus_client::registry::Registry>,
}

pub struct AxumMetricsApp {
    listen_addr: String,
    prometheus_registry: Arc<prometheus_client::registry::Registry>,
}

impl AxumMetricsApp {
    pub fn new(
        listen_addr: String,
        prometheus_registry: Arc<prometheus_client::registry::Registry>,
    ) -> Self {
        Self {
            listen_addr,
            prometheus_registry,
        }
    }

    pub async fn run<F>(self: AxumMetricsApp, shutdown_signal: F) -> Result<(), std::io::Error>
    where
        F: Future<Output = ()> + Send + 'static,
    {
        let listener = tokio::net::TcpListener::bind(&self.listen_addr).await?;
        tracing::info!("listening on http://{}", listener.local_addr()?);

        axum::serve(listener, self.into_router())
            .with_graceful_shutdown(shutdown_signal)
            .await
    }

    pub fn into_router(self) -> Router {
        let state = MetricState {
            prometheus_registry: self.prometheus_registry,
        };

        Router::new()
            .route("/metrics", get(metrics))
            .with_state(state)
    }
}

async fn metrics(State(state): State<MetricState>) -> Result<impl IntoResponse, Error> {
    let mut buffer = String::new();
    prometheus_client::encoding::text::encode(&mut buffer, &state.prometheus_registry)
        .map_err(|e| Error::new(ErrorKind::Internal).with_source(e))?;

    let response = Response::builder()
        .status(StatusCode::OK)
        .header(
            header::CONTENT_TYPE,
            "application/openmetrics-text; version=1.0.0; charset=utf-8",
        )
        .body(axum::body::Body::from(buffer))
        .map_err(|e| Error::new(ErrorKind::Internal).with_source(e))?;

    Ok(response)
}

#[cfg(test)]
mod tests {
    use crate::{AxumMetricsApp, Config, metrics::create_registry};
    use std::sync::Arc;

    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt; // for `call`, `oneshot`, `ready`, and `collect`

    #[tokio::test]
    async fn test_metrics_get() {
        let cfg = Config::default();
        let prometheus_registry = create_registry("testing-123".to_string()).unwrap();
        let app = AxumMetricsApp::new(cfg.metrics_listen_addr, Arc::new(prometheus_registry))
            .into_router();
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body_str = std::str::from_utf8(&body).unwrap();
        assert!(body_str.contains(r#"subbers_build_info{version="testing-123""#));
    }
}
