use super::icon_templates::{play_svg, stop_svg};
use crate::{GameState, GameView, PlayerView};
use maud::{Markup, PreEscaped, html};

pub fn list_games(games: &[GameView]) -> Markup {
    let rows: Vec<Markup> = games.iter().map(game_table_row).collect();
    html! {
        h2 class="small" { "Games" }
        (game_table(rows))
    }
}

fn game_table(rows: Vec<Markup>) -> Markup {
    html! {
        table class="table" {
            thead {
                tr {
                    th { "#" }
                    th { "Started" }
                    th { "End" }
                    th { "Total" }
                    th { "Current" }
                    th { "Period(s)" }
                    th { "MVP" }
                }
            }
            tbody {
                tr #new_game {
                    td {
                        button
                            class="primary small small-elevate"
                            type="button"
                            hx-post="/games"
                            hx-target="#new_game"
                            hx-swap="afterend"
                        { "new" }
                    }
                }
                @for row in rows { (row) }
            }
        }
    }
}

pub fn game_table_row(game: &GameView) -> Markup {
    html! {
        tr {
            td {
                a href=(format!("/games/{}", game.id)) {
                    button class="primary small small-elevate" type="button" { (game.id) }
                }
            }
            (PreEscaped(game_table_row_columns(game)))
        }
    }
}

// maudfmt doesn't support multiple match patterns, so build a slice of table columns and then
// convert to html and return at the end.
pub fn game_table_row_columns(game: &GameView) -> String {
    let mut columns = vec![];

    let ph = "-".to_string();

    // Started
    match game.state {
        GameState::NotStarted => {
            columns.push(ph.clone());
        }
        GameState::InProgress | GameState::Paused | GameState::Finished => {
            columns.push(game.start_time_as_digital());
        }
    }

    // End
    match game.state {
        GameState::NotStarted | GameState::InProgress | GameState::Paused => {
            columns.push(ph.clone());
        }
        GameState::Finished => {
            columns.push(game.end_time_as_digital());
        }
    }
    // Total
    match game.state {
        GameState::NotStarted => {
            columns.push(ph.clone());
        }
        GameState::InProgress | GameState::Paused | GameState::Finished => {
            columns.push(game.total_duration());
        }
    }
    // Current
    match game.state {
        GameState::NotStarted => {
            columns.push(ph.clone());
        }
        GameState::InProgress => {
            columns.push(game.current_period_duration());
        }
        GameState::Paused | GameState::Finished => {
            columns.push("0s".to_string());
        }
    }

    // Period
    columns.push(game.periods.len().to_string());

    // MVP
    columns.push(
        game.mvp
            .and_then(|pid| {
                game.players
                    .iter()
                    .find(|p| p.id == pid)
                    .map(|p| p.name.clone())
            })
            .unwrap_or_else(|| ph.clone()),
    );

    // Return
    columns
        .iter()
        .map(|v| format!("<td>{}</td>", v))
        .collect::<Vec<String>>()
        .concat()
}

fn game_action_table(rows: Vec<Markup>) -> Markup {
    html! {
        table class="table" {
            thead {
                tr {
                    th { "#" }
                    th { "Started" }
                    th { "End" }
                    th { "Total" }
                    th { "Current" }
                    th { "Period(s)" }
                    th { "MVP" }
                }
            }
            tbody #games {
                @for row in rows { (row) }
            }
        }
    }
}

pub fn game_action_table_row(game: &GameView) -> Markup {
    let base_path = format!("/games/{}/", game.id);
    html! {
        tr {
            td { (game.id) }
            td { // Started
                @match game.state {
                    GameState::NotStarted => {
                        button class="primary small small-elevate"
                            type="button"
                            hx-post={ (base_path) "start" }
                            hx-target="#game"
                            hx-swap="outerHTML" { (play_svg()) }
                    }
                    GameState::InProgress | GameState::Paused | GameState::Finished => {
                        (game.start_time_as_digital())
                    }
                }
            }
            td { // End
                @match game.state {
                    GameState::NotStarted => { "-" }
                    GameState::InProgress | GameState::Paused => {
                        button class="primary small small-elevate error"
                            type="button"
                            hx-post={ (base_path) "end" }
                            hx-target="#game"
                            hx-swap="outerHTML" { (stop_svg()) }
                    }
                    GameState::Finished => { (game.end_time_as_digital()) }
                }
            }
            td { // Total
                @match game.state {
                    GameState::NotStarted => { "-" }
                    GameState::InProgress | GameState::Paused | GameState::Finished => {
                        (game.total_duration())
                    }
                }
            }
            td { // Current
                @match game.state {
                    GameState::NotStarted => { "-" }
                    GameState::InProgress => { (game.current_period_duration()) }
                    GameState::Paused | GameState::Finished => { "0s" }
                }
            }
            td { // Period
                @match game.state {
                    GameState::InProgress => {
                        button class="primary small small-elevate error"
                            type="button"
                            hx-post={ (base_path) "end-period" }
                            hx-target="#game"
                            hx-swap="outerHTML"  { (stop_svg()) }
                    }
                    GameState::Paused => {
                        button class="primary small small-elevate"
                            type="button"
                            hx-post={ (base_path) "start-period" }
                            hx-target="#game"
                            hx-swap="outerHTML"  { (play_svg()) }
                    }
                    GameState::NotStarted | GameState::Finished => { (game.periods.len()) }
                }
            }
            td {  // MVP
                (mvp_select(game.id, &game.players, game.mvp))
            }
        }
    }
}

fn mvp_select(game_id: u32, players: &Vec<PlayerView>, mvp: Option<u32>) -> Markup {
    let base_path = format!("/games/{}/mvp", game_id);
    let mvp_set = mvp.is_some();
    html! {
        div class="field border" {
            select
                name="player_id"
                id="player_id"
                hx-put=(base_path)
                hx-swap="none"
                hx-trigger="input changed"
            {
                option value="-" selected[!mvp_set] class="center-align" { "-" }
                @for player in players {
                    @let current = mvp == Some(player.id);
                    option value=(player.id) selected[current] { (player.name) }
                }
            }
        }
    }
}

pub fn get_game(game: &GameView, players: Markup) -> Markup {
    let rows = vec![game_action_table_row(game)];
    let base_path = format!("/games/{}", game.id);
    let poll = match game.state {
        GameState::InProgress | GameState::Paused => true,
        GameState::NotStarted | GameState::Finished => false,
    };
    html! {
        div id="game" hx-get=(base_path) hx-trigger={ "every 5s [" (poll) "]" } hx-swap="outerHTML" {
            h2 class="small" { "Game " (game.id) }

            (game_action_table(rows))
            h3 class="small" { "Players" }
            (players)
        }
    }
}
