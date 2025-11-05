use super::icon_templates::{pause_svg, play_svg};
use crate::{GameState, PlayerView};
use maud::{Markup, html};

pub fn list_players(players: &[PlayerView]) -> Markup {
    let rows: Vec<Markup> = players.iter().map(player_table_row).collect();
    html! {
        h2 class="small" { "Players" }
        (new_player_form())
        (player_table(rows))
    }
}

fn new_player_form() -> Markup {
    html! {
        form
            hx-post="/players"
            hx-target="#players"
            hx-swap="afterbegin"
            hx-on::after-request="if(event.detail.successful) this.reset()"
        {
            fieldset {
                div class="grid" {
                    div class="s12 m6 l3" {
                        // class="active" https://github.com/beercss/beercss/issues/274
                        div class="field border label" {
                            input
                                type="number"
                                pattern="\\d+"
                                placeholder="1"
                                name="number"
                                required=""
                                class="active" {}
                            label for="number" class="active" { "Number" }
                        }
                    }
                    div class="s12 m6 l3" {
                        div class="field border label" {
                            input
                                type="text"
                                name="name"
                                placeholder="Baller"
                                required=""
                                class="active" {}
                            label for="name" class="active" { "Name" }
                        }
                    }
                    div class="s12 m6 l3" {
                        div class="field middle-align" {
                            button type="submit" class="primary small small-elevate" {
                                "Create Player"
                            }
                        }
                    }
                }
            }
        }
    }
}

fn player_table(rows: Vec<Markup>) -> Markup {
    html! {
        table class="table" hx-target="closest tr" hx-swap="outerHTML" {
            thead {
                tr {
                    th { "#" }
                    th { "Name" }
                    th { "Count" }
                    th { "Total" }
                    th { "Edit" }
                }
            }
            tbody #players {
                @for row in rows { (row) }
            }
        }
    }
}

pub fn player_table_row(player: &PlayerView) -> Markup {
    let base_path = format!("/players/{}/edit", player.id);
    html! {
        tr {
            td { (player.number) }
            td { (player.name) }
            td { (player.play_count) }
            td { (player.total_duration()) }
            td {
                button class="btn danger" type="button" hx-get=(base_path) hx-trigger="click" {
                    "EDIT"
                }
            }
        }
    }
}

pub fn player_edit_table_row(player: &PlayerView) -> Markup {
    let base_path = format!("/players/{}", player.id);
    html! {
        tr {
            td {
                div class="field border" {
                    input type="number" pattern="\\d+" autofocus name="number" value=(player.number) {}
                }
            }
            td {
                div class="field border" {
                    input type="text" name="name" value=(player.name) {}
                }
            }
            td { (player.play_count) }
            td { (player.total_duration()) }
            td {
                button class="btn danger" type="button" hx-get=(base_path) { "Cancel" }
                button class="btn danger" type="button" hx-put=(base_path) hx-include="closest tr" {
                    "Save"
                }
            }
        }
    }
}

pub fn player_actions(game_id: &u32, game_state: &GameState, players: &[PlayerView]) -> Markup {
    let rows: Vec<Markup> = players
        .iter()
        .map(|p| player_actions_table_row(game_id, game_state, p))
        .collect();
    html! {
        (player_actions_table(rows))
    }
}

fn player_actions_table(rows: Vec<Markup>) -> Markup {
    html! {
        table class="table" {
            thead {
                tr {
                    th { "#" }
                    th { "Name" }
                    th { "Count" }
                    th { "Total" }
                    th { "Current" }
                    th { "Sub" }
                }
            }
            tbody {
                @for row in rows { (row) }
            }
        }
    }
}

pub fn player_actions_table_row(
    game_id: &u32,
    game_state: &GameState,
    player: &PlayerView,
) -> Markup {
    html! {
        tr {
            td { (player.number) }
            td { (player.name) }
            td { (player.play_count) }
            td { (player.total_duration()) }
            td { (player.current_period_duration()) }
            td { (sub_button(game_id, game_state, &player.id, player.playing)) }
        }
    }
}

fn sub_button(game_id: &u32, game_state: &GameState, player_id: &u32, playing: bool) -> Markup {
    let base_path = format!("/games/{}/players/{}/", game_id, player_id);
    // maudfmt panics on @match with | so use rusts match versus maud's @match.
    match game_state {
        GameState::NotStarted | GameState::Paused | GameState::Finished => html! {
            "-"
        },
        GameState::InProgress => match playing {
            true => html! {
                button
                    class="primary small small-elevate error"
                    type="button"
                    hx-post={ (base_path) "sub-off" }
                    hx-target="closest tr"
                    hx-swap="outerHTML"
                { (pause_svg()) }
            },
            false => html! {
                button
                    class="primary small small-elevate"
                    type="button"
                    hx-post={ (base_path) "sub-on" }
                    hx-target="closest tr"
                    hx-swap="outerHTML"
                { (play_svg()) }
            },
        },
    }
}
