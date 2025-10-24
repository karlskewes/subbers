# subbers

Manage subs for your sports game.

Run the app locally on a laptop or home server and then access from your mobile
phone at the game via Tailscale VPN or similar.

## Getting Started

#### Start app

```
cargo run
```

[View app](http://localhost:8080/)

Alternatively:

```
$ cargo run -- --help
    Finished `dev` profile [unoptimized + debuginfo] target(s) in 0.08s
     Running `target/debug/subbers --help`
Manage your sports game subs

Usage: subbers [OPTIONS]

Options:
  -s, --sqlite-filepath <SQLITE_FILEPATH>  SQLite file path [default: subbers.sql]
  -l, --listen-addr <LISTEN_ADDR>          Listen Address for HTTP server [default: 0.0.0.0:8080]
  -b, --basic-auth <BASIC_AUTH>            Basic Auth 'user:pass' for HTTP server
  -h, --help                               Print help
  -V, --version                            Print version
```

#### Create Players

All created players are available to sub in games.

Edit a player to change their name and number. Past games are not updated to the new name/number.

#### Create Game

In the games table click `new` then click on the game number to manage it.

#### Game Actions

| Table  | Column  | Action | Outcome                                                                                                    |
| ------ | ------- | ------ | ---------------------------------------------------------------------------------------------------------- |
| Game   | Started | Play   | begin the game timer and the first period. Page refreshes every 5 seconds.                                 |
| Game   | Ended   | Stop   | stop the game timer and sub off all players. Players game statistics are added to their global statistics. |
| Game   | Period  | Play   | start the period, enable players to sub on.                                                                |
| Game   | Period  | Stop   | stop the period and sub all players off. Page stops refreshing.                                            |
| Game   | MVP     | Select | upsert MVP.                                                                                                |
| Player | Sub     | Play   | sub player on, increasing play count and starting duration timer.                                          |
| Player | Sub     | Pause  | sub player off.                                                                                            |

#### Deleting Players and Games

TODO.

Use [DB Browser for SQLite](https://sqlitebrowser.org/) or similar to edit the SQL file directly.

## Cross compiling

If you have [nix](https://github.com/NixOS/nix) installed, you can cross
compile via:

```sh
nix build .#subbers-x86_64-linux

nix build .#subbers-aarch64-linux
```

## Contributing

This project was built to manage my son's basketball games and to experiment
with different software libraries and code patterns.

Whilst the project is feature complete for my use case there are many
directions it could take. Please feel free to fork or create an issue to
discuss.

## Thanks

Creators of the following:

- [Beer CSS](https://github.com/beercss/beercss)
- [Maud](https://github.com/lambda-fairy/maud)
- [axum](https://github.com/tokio-rs/axum)
- [htmx](https://htmx.org/)
- [Rust](https://www.rust-lang.org/)
