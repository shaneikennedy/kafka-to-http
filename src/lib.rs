use kafka::consumer::{Consumer, FetchOffset, GroupOffsetStorage};
use rayon::ThreadPool;
use reqwest::blocking::{Client, Response};
use serde::Deserialize;
use serde_yaml::{self};
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub consumer: ConsumerConfig,
    pub http: HttpConfig,
}

#[derive(Debug, Deserialize)]
pub struct ConsumerConfig {
    pub host: String,
    pub topic: String,
    pub app_name: String,
    pub pool_size: u32,
}

#[derive(Debug, Deserialize)]
pub struct HttpConfig {
    pub host: String,
    pub endpoint: String,
    pub timeout: u64,
}

impl Config {
    pub fn build(args: &[String]) -> Result<Config, &'static str> {
        if args.len() < 2 {
            return Err("Not enough arguments");
        }
        let config_file = args[1].clone();
        let f = std::fs::File::open(config_file).expect("Could not open file.");
        let config: Config = serde_yaml::from_reader(f).expect("Could not read values.");
        Ok(config)
    }
}

pub struct ConsumerWrapper {
    pub consumer: Consumer,
    pub worker_pool: ThreadPool,
    pub handler: MessageHandler,
}

impl ConsumerWrapper {
    pub fn build(
        config: ConsumerConfig,
        message_handler: MessageHandler,
    ) -> Result<ConsumerWrapper, &'static str> {
        let consumer = Consumer::from_hosts(vec![config.host])
            .with_topic(config.topic)
            .with_fallback_offset(FetchOffset::Latest)
            .with_group(config.app_name.to_owned())
            .with_offset_storage(GroupOffsetStorage::Kafka)
            .create()
            .unwrap();

        let worker_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(config.pool_size.try_into().unwrap())
            .build()
            .unwrap();

        return Ok(ConsumerWrapper {
            consumer,
            worker_pool,
            handler: message_handler,
        });
    }

    pub fn consume(&mut self) {
        loop {
            self.worker_pool.scope(|s| {
                for ms in self.consumer.poll().unwrap().iter() {
                    for m in ms.messages() {
                        let data = Vec::from(m.value);
                        s.spawn(|_| self.handler.handle(data));
                    }
                    let _ = self.consumer.consume_messageset(ms);
                }
                self.consumer.commit_consumed().unwrap();
            })
        }
    }
}

pub struct MessageHandler {
    client: Client,
    msg_destination: String,
    timeout_policy: Duration,
}

impl MessageHandler {
    pub fn build(http_config: HttpConfig) -> MessageHandler {
        MessageHandler {
            client: reqwest::blocking::Client::new(),
            msg_destination: format!("{0}/{1}", http_config.host, http_config.endpoint),
            timeout_policy: Duration::new(http_config.timeout, 0),
        }
    }

    pub fn handle(&self, data: Vec<u8>) {
        let request_defaults = self.setup_http_request();
        let body = String::from_utf8(data.to_vec()).expect("expecting string body");
        println!("{:#?}", body);
        let request = request_defaults.body(body).send();
        if request.is_ok() {
            let resp = request.ok();
            match resp {
                Some(r) => self.handle_response(r),
                None => self.handle_request_error(),
            }
        } else {
            self.handle_request_error();
        }
    }

    fn setup_http_request(&self) -> reqwest::blocking::RequestBuilder {
        self.client
            .post(self.msg_destination.clone())
            .timeout(self.timeout_policy)
    }

    fn handle_request_error(&self) {
        println!("what the fuck");
    }

    fn handle_response(&self, response: Response) {
        print!("Received {:?} ", response.status());
        match response.status().as_u16() {
            s if s >= 200 && s < 400 => println!("Message delivered successfully"),
            s if s >= 400 && s < 500 => println!("Client error, check your request"),
            s if s >= 500 => println!("Server error"),
            _ => println!("received unhanlded status code"),
        }
    }
}
