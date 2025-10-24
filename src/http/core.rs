//! `http` wraps the domain service and provides http endpoints for interacting with the Service.

use super::{games_templates, layout_templates, players_templates};
use crate::{Error, GameView, Service, into_game_views, into_player_views};
use axum::{
    Router,
    extract::{Form, FromRequestParts, Path, State},
    http::{StatusCode, header, request::Parts},
    middleware::{self},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post, put},
};
use base64::prelude::*;
use maud::Markup;
use serde::Deserialize;
use std::sync::Arc;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match self {
            Self::InvalidInput(_) => StatusCode::BAD_REQUEST,
            Self::NotFound => StatusCode::NOT_FOUND,
            Self::Conflict => StatusCode::CONFLICT,
            Self::Internal(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = self.to_string();

        (status, body).into_response()
    }
}

#[derive(Clone)]
struct AuthConfig {
    basic_auth: Option<User>, // could make hashmap with a User struct, name/pass/role.
}

impl AuthConfig {
    fn new(basic_auth: Option<User>) -> Self {
        Self { basic_auth }
    }

    // validate checks the provided AUTHORIZATION header value matches any configured auth values.
    fn validate(&self, user: Option<User>) -> bool {
        match (&self.basic_auth, user) {
            (None, _) => true,
            (Some(_), None) => false,
            (Some(want), Some(got)) => {
                want.username == got.username && want.password == got.password
            }
        }
    }
}

pub struct AxumApp {
    // tracing, etc
    basic_auth: Option<User>,
    listen_addr: String,
    svc: Service,
}

impl AxumApp {
    #[must_use]
    pub fn new(listen_addr: String, basic_auth: Option<User>, svc: Service) -> Self {
        Self {
            listen_addr,
            basic_auth,
            svc,
        }
    }

    pub async fn run<F>(self: AxumApp, shutdown_signal: F) -> Result<(), std::io::Error>
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
        let state = AppState { svc: self.svc };

        let auth_config = Arc::new(AuthConfig::new(self.basic_auth));

        Router::new()
            .route("/", get(home))
            .route("/static/{filename}", get(assets))
            // game
            .route("/games", get(list_games).post(create_game))
            .route("/games/{game_id}", get(get_game))
            .route("/games/{game_id}/start", post(start_game))
            .route("/games/{game_id}/end", post(end_game))
            .route("/games/{game_id}/start-period", post(start_game_period))
            .route("/games/{game_id}/end-period", post(end_game_period))
            .route("/games/{game_id}/mvp", put(upsert_mvp))
            .route(
                "/games/{game_id}/players/{player_id}/sub-on",
                post(sub_player_on),
            )
            .route(
                "/games/{game_id}/players/{player_id}/sub-off",
                post(sub_player_off),
            )
            // players
            .route("/players", get(list_players).post(create_player))
            .route(
                "/players/{player_id}",
                get(get_player).put(edit_player).delete(delete_player),
            )
            .route("/players/{player_id}/edit", get(edit_player_form))
            // basic auth required for above route(s)
            .route_layer(middleware::from_extractor_with_state::<RequireAuth, _>(
                auth_config.clone(),
            ))
            // state (db, etc)
            .with_state(state.clone())
            // basic auth not required for below routes
            .route("/ready", get(ready))
    }
}

#[derive(Clone)]
struct AppState {
    svc: Service,
}

#[derive(Debug, Deserialize)]
struct NewPlayerForm {
    pub name: String,
    pub number: u32,
}

#[derive(Clone)]
pub struct User {
    pub username: String,
    pub password: String,
}

// RequireAuth implements the Axum Extractor to perform authorization.
#[derive(Clone)]
struct RequireAuth {}

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
            .and_then(|hv| decode_basic_auth(hv.to_string()));

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

async fn ready() -> impl IntoResponse {
    StatusCode::OK
}

async fn home(State(state): State<AppState>) -> Result<impl IntoResponse, Error> {
    let title = "subbers";
    let description = "Manage your sports game subs";

    let games = into_game_views(state.svc.list_games()?);
    let players = into_player_views(state.svc.list_players()?);

    let games_html = games_templates::list_games(&games);
    let players_html = players_templates::list_players(&players);
    let contents = layout_templates::games_players(&games_html, &players_html);
    let body = Html(layout_templates::page(title, description, &contents).into_string());

    Ok((StatusCode::OK, body))
}

async fn get_player(
    State(state): State<AppState>,
    Path(player_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let player_id: u32 = player_id
        .trim()
        .parse::<u32>()
        .map_err(|_| Error::InvalidInput("id must be a number".to_string()))?;

    let player = state.svc.get_player(&player_id)?;
    let body = Html(players_templates::player_table_row(&player.into()).into_string());

    Ok((StatusCode::OK, body))
}

async fn create_player(
    State(state): State<AppState>,
    Form(input): Form<NewPlayerForm>,
) -> Result<impl IntoResponse, Error> {
    let player = state.svc.create_player(input.number, input.name)?;

    let body = Html(players_templates::player_table_row(&player.clone().into()).into_string());

    Ok((StatusCode::CREATED, body))
}

async fn edit_player_form(
    State(state): State<AppState>,
    Path(player_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let player_id: u32 = player_id
        .trim()
        .parse::<u32>()
        .map_err(|_| Error::InvalidInput("id must be a number".to_string()))?;

    let player = state.svc.get_player(&player_id)?;
    let body = Html(players_templates::player_edit_table_row(&player.into()).into_string());

    Ok((StatusCode::OK, body))
}

async fn edit_player(
    State(state): State<AppState>,
    Path(player_id): Path<String>,
    Form(input): Form<NewPlayerForm>,
) -> Result<impl IntoResponse, Error> {
    let player_id: u32 = player_id
        .trim()
        .parse::<u32>()
        .map_err(|_| Error::InvalidInput("id must be a number".to_string()))?;

    let mut player = state.svc.get_player(&player_id)?;
    player.name = input.name;
    player.number = input.number;
    state.svc.update_player(player.clone())?;

    let body = Html(players_templates::player_table_row(&player.into()).into_string());

    Ok((StatusCode::OK, body))
}

async fn delete_player(
    State(state): State<AppState>,
    Path(player_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let player_id: u32 = player_id
        .trim()
        .parse::<u32>()
        .map_err(|_| Error::InvalidInput("id must be a number".to_string()))?;

    state.svc.delete_player(&player_id)?;

    Ok(Redirect::to("/players"))
}

async fn list_players(State(state): State<AppState>) -> Result<impl IntoResponse, Error> {
    let players = into_player_views(state.svc.list_players()?);
    let body = Html(players_templates::list_players(&players).into_string());

    Ok((StatusCode::OK, body))
}

// assets serves files embedded into the application. With 'cargo watch -x run' it's not
// too awkward for development and is simple for deployment.
async fn assets(Path(filename): Path<String>) -> Result<impl IntoResponse, Error> {
    let (content_type, body) = match filename.as_str() {
        "beer_3.11.33.min.css" => (
            mime::TEXT_CSS.as_ref(),
            include_str!("./assets/beer_3.11.33.min.css"),
        ),
        "beer_3.11.33.min.js" => (
            mime::APPLICATION_JAVASCRIPT.as_ref(),
            include_str!("./assets/beer_3.11.33.min.js"),
        ),
        "htmx_2.0.4.js" => (
            mime::APPLICATION_JAVASCRIPT.as_ref(),
            include_str!("./assets/htmx_2.0.4.js"),
        ),
        "robots.txt" => (
            mime::TEXT_PLAIN.as_ref(),
            include_str!("./assets/robots.txt"),
        ),
        "theme.css" => (mime::TEXT_CSS.as_ref(), include_str!("./assets/theme.css")),
        _ => return Err(Error::NotFound),
    };

    Ok(([(header::CONTENT_TYPE, content_type)], body))
}

async fn create_game(State(state): State<AppState>) -> Result<impl IntoResponse, Error> {
    let game: GameView = state.svc.create_game()?.into();
    let body = Html(games_templates::game_table_row(&game).into_string());

    Ok((StatusCode::CREATED, body))
}

async fn list_games(State(state): State<AppState>) -> Result<impl IntoResponse, Error> {
    let games = into_game_views(state.svc.list_games()?);
    let body = Html(games_templates::list_games(&games).into_string());

    Ok((StatusCode::OK, body))
}

fn get_game_html(game: GameView) -> Markup {
    let player_actions = players_templates::player_actions(&game.id, &game.state, &game.players);
    games_templates::get_game(&game, player_actions)
}

async fn get_game(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
    headers: header::HeaderMap,
) -> Result<impl IntoResponse, Error> {
    let game_id: u32 = game_id
        .trim()
        .parse::<u32>()
        .map_err(|_| Error::InvalidInput("id must be a number".to_string()))?;

    let game: GameView = state.svc.get_game(&game_id)?.into();
    let contents = get_game_html(game);
    let body: String;

    if headers.contains_key("HX-Request") {
        // body will be injected into an existing page.
        body = contents.into_string();
    } else {
        let title = format!("Game {}", &game_id);
        let description = format!("Game {}", &game_id);
        body = layout_templates::page(&title, &description, &contents).into_string();
    }

    Ok((StatusCode::OK, Html(body)))
}

async fn start_game(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let game_id: u32 = game_id
        .trim()
        .parse::<u32>()
        .map_err(|_| Error::InvalidInput("id must be a number".to_string()))?;

    let game: GameView = state.svc.start_game(&game_id)?.into();
    let body = get_game_html(game).into_string();

    Ok((StatusCode::OK, body))
}

async fn end_game(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let game_id: u32 = game_id
        .trim()
        .parse::<u32>()
        .map_err(|_| Error::InvalidInput("id must be a number".to_string()))?;

    let game: GameView = state.svc.end_game(&game_id)?.into();
    let body = get_game_html(game).into_string();

    Ok((StatusCode::OK, body))
}

async fn start_game_period(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let game_id: u32 = game_id
        .trim()
        .parse::<u32>()
        .map_err(|_| Error::InvalidInput("id must be a number".to_string()))?;

    let game: GameView = state.svc.start_game_period(&game_id)?.into();
    let body = get_game_html(game).into_string();

    Ok((StatusCode::OK, body))
}

async fn end_game_period(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let game_id: u32 = game_id
        .trim()
        .parse::<u32>()
        .map_err(|_| Error::InvalidInput("id must be a number".to_string()))?;

    let game: GameView = state.svc.end_game_period(&game_id)?.into();
    let body = get_game_html(game).into_string();

    Ok((StatusCode::OK, body))
}

#[derive(Debug, Deserialize)]
struct GameMVPForm {
    pub player_id: u32,
}

async fn upsert_mvp(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
    Form(input): Form<GameMVPForm>,
) -> Result<impl IntoResponse, Error> {
    let game_id: u32 = game_id
        .trim()
        .parse::<u32>()
        .map_err(|_| Error::InvalidInput("game id must be a number".to_string()))?;

    let game: GameView = state.svc.upsert_mvp(&game_id, &input.player_id)?.into();
    let body = get_game_html(game).into_string();

    Ok((StatusCode::OK, body))
}

async fn sub_player_on(
    State(state): State<AppState>,
    Path((game_id, player_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let game_id: u32 = game_id
        .trim()
        .parse::<u32>()
        .map_err(|_| Error::InvalidInput("game id must be a number".to_string()))?;

    let player_id: u32 = player_id
        .trim()
        .parse::<u32>()
        .map_err(|_| Error::InvalidInput("player id must be a number".to_string()))?;

    let game: GameView = state.svc.sub_player_on(&game_id, &player_id)?.into();
    let player = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .ok_or_else(|| Error::Internal("player not found".to_string()))?;

    let body = Html(
        players_templates::player_actions_table_row(&game.id, &game.state, &player).into_string(),
    );

    Ok((StatusCode::OK, body))
}

async fn sub_player_off(
    State(state): State<AppState>,
    Path((game_id, player_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let game_id: u32 = game_id
        .trim()
        .parse::<u32>()
        .map_err(|_| Error::InvalidInput("game id must be a number".to_string()))?;

    let player_id: u32 = player_id
        .trim()
        .parse::<u32>()
        .map_err(|_| Error::InvalidInput("player id must be a number".to_string()))?;

    let game: GameView = state.svc.sub_player_off(&game_id, &player_id)?.into();
    let player = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .ok_or_else(|| Error::Internal("player not found".to_string()))?;

    let body = Html(
        players_templates::player_actions_table_row(&game.id, &game.state, &player).into_string(),
    );

    Ok((StatusCode::OK, body))
}

#[cfg(test)]
mod tests {
    use crate::{AxumApp, Config, InMemoryRepo, Service};
    use std::sync::Arc;

    use axum::{
        body::Body,
        http::{self, Request, StatusCode},
    };
    use http_body_util::BodyExt;
    use tower::ServiceExt; // for `call`, `oneshot`, `ready`, and `collect`

    #[tokio::test]
    async fn test_get() {
        let cfg = Config::default();
        let repo = Arc::new(InMemoryRepo::new());
        let svc = Service::new(repo);
        let app = AxumApp::new(cfg.listen_addr, None, svc).into_router();

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/players/1")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn test_post() {
        let cfg = Config::default();
        let repo = Arc::new(InMemoryRepo::new());
        let svc = Service::new(repo);
        let app = AxumApp::new(cfg.listen_addr, None, svc).into_router();

        let response = app
            .oneshot(
                Request::builder()
                    .method(http::Method::POST)
                    .uri("/players")
                    .header(
                        http::header::CONTENT_TYPE,
                        mime::APPLICATION_WWW_FORM_URLENCODED.as_ref(),
                    )
                    .body(Body::from("name=foo&number=1"))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CREATED);

        let body = response.into_body().collect().await.unwrap().to_bytes();
        let body = std::str::from_utf8(&body).unwrap();

        assert_eq!(
            body,
            "<tr><td>1</td><td>foo</td><td>0</td><td>0m 0s</td><td>-</td></tr>"
        );
    }
}
