use std::{env, process, time::Duration};

use kafka::consumer::{Consumer, FetchOffset, GroupOffsetStorage};
use kafka_quickstart::{Config, ConsumerConfig, HttpConfig};
use reqwest::blocking::{Client, RequestBuilder, Response};
use threadpool;

fn main() {
    let args: Vec<String> = env::args().collect();
    let config = Config::build(&args).unwrap_or_else(|err| {
        eprintln!("Problems parsing arguments: {err}");
        process::exit(1);
    });

    let mut consumer = init_consumer(config.consumer);
    let client = reqwest::blocking::Client::new();
    let threadpool = threadpool::Builder::new().num_threads(10).build();
    loop {
        for ms in consumer.poll().unwrap().iter() {
            for m in ms.messages() {
                let request_defaults = setup_http_request(&client, &config.http);
                let body = String::from_utf8(m.value.to_vec()).expect("expecting string body");
                println!("{:#?}", body);
                threadpool.execute(|| handle_message(request_defaults, body));
            }
            let _ = consumer.consume_messageset(ms);
        }
        consumer.commit_consumed().unwrap();
    }
}

fn init_consumer(config: ConsumerConfig) -> Consumer {
    Consumer::from_hosts(vec![config.host])
        .with_topic(config.topic)
        .with_fallback_offset(FetchOffset::Latest)
        .with_group(config.app_name.to_owned())
        .with_offset_storage(GroupOffsetStorage::Kafka)
        .create()
        .unwrap()
}

fn handle_message(req: RequestBuilder, body: String) {
    let request = req.body(body).send();
    if request.is_ok() {
        let resp = request.ok();
        match resp {
            Some(r) => handle_response(r),
            None => handle_request_error(),
        }
    } else {
        handle_request_error();
    }
}

fn setup_http_request(client: &Client, config: &HttpConfig) -> reqwest::blocking::RequestBuilder {
    let msg_dst = format!("{0}/{1}", config.host, config.endpoint);
    client
        .post(msg_dst)
        .timeout(Duration::new(config.timeout, 0))
}

fn handle_request_error() {
    println!("what the fuck");
}

fn handle_response(response: Response) {
    print!("Received {:?} ", response.status());
    match response.status().as_u16() {
        s if s >= 200 && s < 400 => println!("Message delivered successfully"),
        s if s >= 400 && s < 500 => println!("Client error, check your request"),
        s if s >= 500 => println!("Server error"),
        _ => println!("received unhanlded status code"),
    }
}
