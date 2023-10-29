use std::{env, process};

use kafka_to_http::{Config, ProxyApplication};

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::build(&args).unwrap_or_else(|err| {
        eprintln!("Problems parsing arguments: {err}");
        process::exit(1);
    });
    let app = ProxyApplication::build(config).expect("Error configuring proxy application");
    app.start();
}
