use maud::{DOCTYPE, Markup, html};

fn header(title: &str, description: &str) -> Markup {
    html! {
        meta charset="utf-8"
        head {
            title { (title) }
            link rel="stylesheet" href="/static/beer_3.11.33.min.css";
            link rel="stylesheet" href="/static/theme.css";
            link rel="icon" href="/static/favicon.ico" type="image/x-icon";
            meta charset="UTF-8";
            meta name="viewport" content="width=device-width, initial-scale=1.0";
            meta name="author" content="Karl Skewes";
            meta name="copyright" content="© 2025 Karl Skewes";
            meta name="description" content=(description);
            meta http-equiv="X-UA-Compatible" content="ie=edge";
            meta http-equiv="Content-Type" content="text/html; charset=utf-8";
            script src="/static/htmx_2.0.4.js" {};
            script type="module" src="/static/beer_3.11.33.min.js" {};
        }
    }
}

fn footer() -> Markup {
    html! {
        footer {
        }
    }
}

pub fn body(contents: &Markup) -> Markup {
    html! {
        body class="light" { // TODO: use OS/browser default mode light/dark.
            header class="responsive" {
                nav {
                    a class="max center-align" href="/" { h6 { "subbers" } }
                }
                hr;
            }

            main class="responsive" { (contents) }
        }
    }
}

pub fn games_players(games: &Markup, players: &Markup) -> Markup {
    html! {
        (games)
        hr class="large";
        (players)
    }
}

pub fn page(title: &str, description: &str, contents: &Markup) -> Markup {
    html! {
        (DOCTYPE)
        // Add the header markup to the page
        (header(title, description))
        (body(contents))
        (footer())
    }
}
