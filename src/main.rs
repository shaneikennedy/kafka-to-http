use std::{env, process};

use kafka_to_http::{Config, ProxyApplication};
use log::error;
use simple_logger::SimpleLogger;

fn main() {
    SimpleLogger::new()
        .env()
        .with_level(log::LevelFilter::Info)
        .with_utc_timestamps()
        .init()
        .unwrap();
    let args: Vec<String> = env::args().collect();
    let config = Config::build(&args).unwrap_or_else(|err| {
        error!("Problems parsing arguments: {err}");
        process::exit(1);
    });
    let app = ProxyApplication::build(config).expect("Error configuring proxy application");
    app.start();
}
