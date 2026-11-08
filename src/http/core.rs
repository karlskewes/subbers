//! `http` wraps the domain service and provides http endpoints for interacting with the Service.

use super::{games_templates, layout_templates, players_templates};
use crate::http::middlewares::{self, AuthConfig, AxumAppMetrics};
use crate::{
    Error, ErrorKind, GameId, GameView, JerseyNumber, PlayerId, Service, into_game_views,
    into_player_views,
};
use axum::{
    Router,
    extract::{Form, Path, State},
    http::{StatusCode, header},
    response::{Html, IntoResponse, Redirect, Response},
    routing::{get, post, put},
};
use maud::Markup;
use serde::Deserialize;

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let status = match self.kind() {
            ErrorKind::InvalidInput(_) => StatusCode::BAD_REQUEST,
            ErrorKind::NotFound => StatusCode::NOT_FOUND,
            ErrorKind::Conflict => StatusCode::CONFLICT,
            ErrorKind::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = self.to_string(); // user message

        // Create error level span instead of Axum's `.on_failure()` default so we can
        // enrich with custom Error fields.
        if status.is_server_error() {
            tracing::error!(status = status.as_u16(), kind = ?self.kind(), context = ?self.context(), location =?self.location(), /*span_trace = %self.span_trace, */"request failed")
        }

        (status, body).into_response()
    }
}

pub struct AxumApp {
    basic_auth: Option<User>,
    listen_addr: String,
    metrics: AxumAppMetrics,
    svc: Service,
}

impl AxumApp {
    #[must_use]
    pub fn new(
        listen_addr: String,
        basic_auth: Option<User>,
        prometheus_registry: &mut prometheus_client::registry::Registry,
        svc: Service,
    ) -> Self {
        let metrics = AxumAppMetrics::new(prometheus_registry);
        // TODO: consider creating optional middlewares (Servicebuilder) here.
        Self {
            listen_addr,
            basic_auth,
            metrics,
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
        let state = AppState {
            svc: self.svc, // struct members (repo) already Arc.
        };

        let auth_middleware = AuthConfig::new(self.basic_auth).into_middleware();

        let router = Router::new()
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
            // middlewares
            .route_layer(auth_middleware)
            // state (db, etc)
            .with_state(state)
            // basic auth not required for below routes
            .route("/ready", get(ready));

        middlewares::instrument_router(router, self.metrics.clone())
    }
}

#[derive(Clone)]
struct AppState {
    svc: Service,
}

#[derive(Debug, Deserialize)]
struct NewPlayerForm {
    pub name: String,
    pub number: String,
}

#[derive(Clone)]
pub struct User {
    pub username: String,
    pub password: String,
}

async fn ready() -> impl IntoResponse {
    StatusCode::OK
}

#[tracing::instrument(skip_all)]
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

#[tracing::instrument(skip_all)]
async fn get_player(
    State(state): State<AppState>,
    Path(player_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let player_id = PlayerId(
        player_id
            .trim()
            .parse::<u32>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput("id must be a number".to_string())))?,
    );

    let player = state.svc.get_player(&player_id)?;
    let body = Html(players_templates::player_table_row(&player.into()).into_string());

    Ok((StatusCode::OK, body))
}

#[tracing::instrument(skip_all)]
async fn create_player(
    State(state): State<AppState>,
    Form(input): Form<NewPlayerForm>,
) -> Result<impl IntoResponse, Error> {
    let number = JerseyNumber::new(&input.number)?;
    let player = state.svc.create_player(number, input.name)?;

    let body = Html(players_templates::player_table_row(&player.clone().into()).into_string());

    Ok((StatusCode::CREATED, body))
}

#[tracing::instrument(skip_all)]
async fn edit_player_form(
    State(state): State<AppState>,
    Path(player_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let player_id = PlayerId(
        player_id
            .trim()
            .parse::<u32>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput("id must be a number".to_string())))?,
    );

    let player = state.svc.get_player(&player_id)?;
    let body = Html(players_templates::player_edit_table_row(&player.into()).into_string());

    Ok((StatusCode::OK, body))
}

#[tracing::instrument(skip_all)]
async fn edit_player(
    State(state): State<AppState>,
    Path(player_id): Path<String>,
    Form(input): Form<NewPlayerForm>,
) -> Result<impl IntoResponse, Error> {
    let player_id = PlayerId(
        player_id
            .trim()
            .parse::<u32>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput("id must be a number".to_string())))?,
    );
    let number = JerseyNumber::new(&input.number)?;

    let mut player = state.svc.get_player(&player_id)?;
    player.name = input.name;
    player.number = number;
    state.svc.update_player(player.clone())?;

    let body = Html(players_templates::player_table_row(&player.into()).into_string());

    Ok((StatusCode::OK, body))
}

#[tracing::instrument(skip_all)]
async fn delete_player(
    State(state): State<AppState>,
    Path(player_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let player_id = PlayerId(
        player_id
            .trim()
            .parse::<u32>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput("id must be a number".to_string())))?,
    );

    state.svc.delete_player(&player_id)?;

    Ok(Redirect::to("/players"))
}

#[tracing::instrument(skip_all)]
async fn list_players(State(state): State<AppState>) -> Result<impl IntoResponse, Error> {
    let players = into_player_views(state.svc.list_players()?);
    let body = Html(players_templates::list_players(&players).into_string());

    Ok((StatusCode::OK, body))
}

// assets serves files embedded into the application. With 'bacon run' it's not
// too awkward for development and is simple for deployment.
#[tracing::instrument(skip_all)]
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
        _ => return Err(Error::new(ErrorKind::NotFound)),
    };

    Ok(([(header::CONTENT_TYPE, content_type)], body))
}

#[tracing::instrument(skip_all)]
async fn create_game(State(state): State<AppState>) -> Result<impl IntoResponse, Error> {
    let game: GameView = state.svc.create_game()?.into();
    let body = Html(games_templates::game_table_row(&game).into_string());

    Ok((StatusCode::CREATED, body))
}

#[tracing::instrument(skip_all)]
async fn list_games(State(state): State<AppState>) -> Result<impl IntoResponse, Error> {
    let games = into_game_views(state.svc.list_games()?);
    let body = Html(games_templates::list_games(&games).into_string());

    Ok((StatusCode::OK, body))
}

fn get_game_html(game: GameView) -> Markup {
    let player_actions = players_templates::player_actions(&game.id, &game.state, &game.players);
    games_templates::get_game(&game, player_actions)
}

#[tracing::instrument(skip_all)]
async fn get_game(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
    headers: header::HeaderMap,
) -> Result<impl IntoResponse, Error> {
    let game_id = GameId(
        game_id
            .trim()
            .parse::<u32>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput("id must be a number".to_string())))?,
    );

    let game: GameView = state.svc.get_game(&game_id)?.into();
    let contents = get_game_html(game);

    let body = if headers.contains_key("HX-Request") {
        // body will be injected into an existing page.
        contents.into_string()
    } else {
        let title = format!("Game {}", game_id);
        let description = format!("Game {}", game_id);

        layout_templates::page(&title, &description, &contents).into_string()
    };

    Ok((StatusCode::OK, Html(body)))
}

#[tracing::instrument(skip_all)]
async fn start_game(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let game_id = GameId(
        game_id
            .trim()
            .parse::<u32>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput("id must be a number".to_string())))?,
    );

    let game: GameView = state.svc.start_game(&game_id)?.into();
    let body = get_game_html(game).into_string();

    Ok((StatusCode::OK, body))
}

#[tracing::instrument(skip_all)]
async fn end_game(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let game_id = GameId(
        game_id
            .trim()
            .parse::<u32>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput("id must be a number".to_string())))?,
    );

    let game: GameView = state.svc.end_game(&game_id)?.into();
    let body = get_game_html(game).into_string();

    Ok((StatusCode::OK, body))
}

#[tracing::instrument(skip_all)]
async fn start_game_period(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let game_id = GameId(
        game_id
            .trim()
            .parse::<u32>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput("id must be a number".to_string())))?,
    );

    let game: GameView = state.svc.start_game_period(&game_id)?.into();
    let body = get_game_html(game).into_string();

    Ok((StatusCode::OK, body))
}

#[tracing::instrument(skip_all)]
async fn end_game_period(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let game_id = GameId(
        game_id
            .trim()
            .parse::<u32>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput("id must be a number".to_string())))?,
    );

    let game: GameView = state.svc.end_game_period(&game_id)?.into();
    let body = get_game_html(game).into_string();

    Ok((StatusCode::OK, body))
}

#[derive(Debug, Deserialize)]
struct GameMVPForm {
    pub player_id: PlayerId,
}

#[tracing::instrument(skip_all)]
async fn upsert_mvp(
    State(state): State<AppState>,
    Path(game_id): Path<String>,
    Form(input): Form<GameMVPForm>,
) -> Result<impl IntoResponse, Error> {
    let game_id = GameId(
        game_id
            .trim()
            .parse::<u32>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput("id must be a number".to_string())))?,
    );

    let game: GameView = state.svc.upsert_mvp(&game_id, &input.player_id)?.into();
    let body = get_game_html(game).into_string();

    Ok((StatusCode::OK, body))
}

#[tracing::instrument(skip_all)]
async fn sub_player_on(
    State(state): State<AppState>,
    Path((game_id, player_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let game_id = GameId(
        game_id
            .trim()
            .parse::<u32>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput("id must be a number".to_string())))?,
    );

    let player_id = PlayerId(player_id.trim().parse::<u32>().map_err(|_| {
        Error::new(ErrorKind::InvalidInput(
            "player id must be a number".to_string(),
        ))
    })?);

    let game: GameView = state.svc.sub_player_on(&game_id, &player_id)?.into();
    let player = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .ok_or_else(|| {
            Error::new(ErrorKind::Internal).with_context("player not found".to_string())
        })?;

    let body = Html(
        players_templates::player_actions_table_row(&game.id, &game.state, player).into_string(),
    );

    Ok((StatusCode::OK, body))
}

#[tracing::instrument(skip_all)]
async fn sub_player_off(
    State(state): State<AppState>,
    Path((game_id, player_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, Error> {
    let game_id = GameId(
        game_id
            .trim()
            .parse::<u32>()
            .map_err(|_| Error::new(ErrorKind::InvalidInput("id must be a number".to_string())))?,
    );

    let player_id = PlayerId(player_id.trim().parse::<u32>().map_err(|_| {
        Error::new(ErrorKind::InvalidInput(
            "player id must be a number".to_string(),
        ))
    })?);

    let game: GameView = state.svc.sub_player_off(&game_id, &player_id)?.into();
    let player = game
        .players
        .iter()
        .find(|p| p.id == player_id)
        .ok_or_else(|| {
            Error::new(ErrorKind::Internal).with_context("player not found".to_string())
        })?;

    let body = Html(
        players_templates::player_actions_table_row(&game.id, &game.state, player).into_string(),
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

        let mut prometheus_registry = prometheus_client::registry::Registry::default();
        let svc = Service::new(repo);
        let app = AxumApp::new(cfg.listen_addr, None, &mut prometheus_registry, svc).into_router();

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
        let mut prometheus_registry = prometheus_client::registry::Registry::default();
        let svc = Service::new(repo);
        let app = AxumApp::new(cfg.listen_addr, None, &mut prometheus_registry, svc).into_router();

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
            "<tr><td>1</td><td>foo</td><td>0</td><td>0m 0s</td><td><button class=\"btn danger\" type=\"button\" hx-get=\"/players/0/edit\" hx-trigger=\"click\">EDIT</button></td></tr>"
        );
    }
}
