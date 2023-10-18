use std::time::Duration;

use kafka::consumer::{Consumer, FetchOffset, GroupOffsetStorage};
use reqwest::blocking::Response;

fn main() {
    let mut consumer = Consumer::from_hosts(vec!["localhost:9092".to_owned()])
        .with_topic("quickstart".to_owned())
        .with_fallback_offset(FetchOffset::Latest)
        .with_group("my-group".to_owned())
        .with_offset_storage(GroupOffsetStorage::Kafka)
        .create()
        .unwrap();
    let destination = "http://localhost:8080";
    let client = reqwest::blocking::Client::new();
    loop {
        for ms in consumer.poll().unwrap().iter() {
            for m in ms.messages() {
                let endpoint = String::from_utf8(m.value.to_vec()).expect("please");
                let request = client
                    .get(format!("{destination}/{endpoint}"))
                    .timeout(Duration::new(2, 0))
                    .send();
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
            let _ = consumer.consume_messageset(ms);
        }
        consumer.commit_consumed().unwrap();
    }
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

fn handle_request_error() {
    println!("what the fuck");
}
