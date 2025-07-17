use std::env;
use std::process;
use subbers::Config;

#[tokio::main]
async fn main() {
    // args() panics if invalid unicode, use std::env::args_os instead if can't panic, but
    // returns OsString which is more complicated.
    let args: Vec<String> = env::args().collect();

    let cfg = Config::build(&args).unwrap_or_else(|err| {
        // print errors to stderr with eprintln! macro instead of println!
        eprintln!("{err}");
        process::exit(1);
    });

    // run returns Result<(), Error> so there's no useful value requiring unwrap when successful.
    if let Err(e) = subbers::run(cfg).await {
        eprintln!("Running failed: {e}");
        process::exit(1);
    };
}
