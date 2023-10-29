use std::{env, process};

use kafka_quickstart::{Config, ConsumerWrapper, MessageHandler};

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::build(&args).unwrap_or_else(|err| {
        eprintln!("Problems parsing arguments: {err}");
        process::exit(1);
    });

    let message_handler = MessageHandler::build(config.http);
    let mut consumer = ConsumerWrapper::build(config.consumer, message_handler).unwrap();
    consumer.consume();
}
