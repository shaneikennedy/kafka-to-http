use std::{env, process};

use kafka_to_http::{Config, MessageHandler, Proxy};

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::build(&args).unwrap_or_else(|err| {
        eprintln!("Problems parsing arguments: {err}");
        process::exit(1);
    });

    let mut proxies: Vec<Proxy> = Vec::new();
    for proxy in config.proxies {
        let message_handler = MessageHandler::build(proxy.http_config);
        let consumer = Proxy::build(proxy.consumer_config, message_handler).unwrap();
        proxies.push(consumer);
    }

    let proxy_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(proxies.len())
        .build()
        .unwrap();

    proxy_pool.scope(|s| {
        for mut proxy in proxies {
            s.spawn(move |_| proxy.start());
        }
    });
}
