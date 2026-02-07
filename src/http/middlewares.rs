use crate::http::User;
use axum::{
    extract::{FromRequestParts, MatchedPath, Request, State},
    http::{StatusCode, header, request::Parts},
    middleware::Next,
    response::{IntoResponse, Response},
};
use base64::prelude::*;
use prometheus_client::encoding::EncodeLabelSet;
use prometheus_client::metrics::{counter::Counter, family::Family, histogram::Histogram};
use std::{sync::Arc, time::Instant};

#[derive(Clone)]
pub struct AuthConfig {
    basic_auth: Option<User>, // could make hashmap with a User struct, name/pass/role.
}

impl AuthConfig {
    pub fn new(basic_auth: Option<User>) -> Self {
        Self { basic_auth }
    }

    pub fn into_middleware(
        self,
    ) -> axum::middleware::FromExtractorLayer<RequireAuth, Arc<AuthConfig>> {
        axum::middleware::from_extractor_with_state::<RequireAuth, _>(Arc::new(self))
    }

    // validate checks the provided AUTHORIZATION header value matches any configured auth values.
    pub fn validate(&self, user: Option<User>) -> bool {
        match (&self.basic_auth, user) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(want), Some(got)) => {
                want.username == got.username && want.password == got.password
            }
        }
    }
}

// RequireAuth implements the Axum Extractor to perform authorization.
#[derive(Clone)]
pub struct RequireAuth {}

// decode_basic_auth will attempt to decode the 'user:pass' and if any errors are present will
// silently return None. TODO: Consider returning HTTP BAD_REQUEST 400's.
fn decode_basic_auth(header_value: String) -> Option<User> {
    let b64 = header_value.strip_prefix("Basic ")?;
    let decoded = BASE64_STANDARD.decode(b64).ok()?;
    let userpass = String::from_utf8(decoded).ok()?;
    let (username, password) = userpass.split_once(':')?;

    Some(User {
        username: username.to_string(),
        password: password.to_string(),
    })
}

impl FromRequestParts<Arc<AuthConfig>> for RequireAuth {
    type Rejection = Response;

    async fn from_request_parts(
        parts: &mut Parts,
        auth_config: &Arc<AuthConfig>,
    ) -> Result<Self, Self::Rejection> {
        let user = parts
            .headers
            .get(header::AUTHORIZATION) // typically only specified once, ignore rest.
            .and_then(|hv| hv.to_str().ok())
            .and_then(|hv| decode_basic_auth(hv.to_string())); // TODO: handle error 400 instead of
        // just returning None.

        // TODO: consider passing parts.uri & parts.method if add roles (admin, viewer).
        if auth_config.validate(user) {
            return Ok(Self {});
        }

        Err((
            StatusCode::UNAUTHORIZED,
            [(
                header::WWW_AUTHENTICATE,
                "Basic realm=\"Credentials required\"",
            )],
        )
            .into_response())
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq, EncodeLabelSet)]
struct RequestLabels {
    method: String,
    path: String,
    status_code: u16,
}

#[derive(Clone)]
pub struct AxumAppMetrics {
    counters: Family<RequestLabels, Counter>,
    histograms: Family<RequestLabels, Histogram>,
}

impl AxumAppMetrics {
    pub fn new(prometheus_registry: &mut prometheus_client::registry::Registry) -> Self {
        let counters = Family::<RequestLabels, Counter>::default();
        let histograms = Family::<RequestLabels, Histogram>::new_with_constructor(|| {
            // good resolution from 5ms to 100ms, then wider spacing to ~2.5s.
            Histogram::new(prometheus_client::metrics::histogram::exponential_buckets(
                0.005, 2.0, 10,
            ))
        });

        prometheus_registry.register("http_requests", "http requests", counters.clone());
        prometheus_registry.register(
            "http_request_duration_seconds",
            "http request duration seconds",
            histograms.clone(),
        );

        Self {
            counters,
            histograms,
        }
    }
}

pub async fn track_metrics(
    State(state): State<AxumAppMetrics>,
    req: Request,
    next: Next,
) -> impl IntoResponse {
    let start = Instant::now();
    // TODO: improve method matching/enum rather than string?
    let method = req.method().clone();
    let path = if let Some(matched_path) = req.extensions().get::<MatchedPath>() {
        matched_path.as_str().to_owned()
    } else {
        req.uri().to_string()
    };

    let response = next.run(req).await;

    let latency = start.elapsed().as_secs_f64();
    let labels = &RequestLabels {
        method: method.to_string(),
        path: path.clone(),
        status_code: response.status().as_u16(),
    };

    state.counters.get_or_create_owned(labels).inc();
    state
        .histograms
        .get_or_create_owned(labels)
        .observe(latency);

    response
}
