use kafka::{
    consumer::{Consumer, FetchOffset, GroupOffsetStorage},
    producer::{Producer, Record, RequiredAcks},
};
use rayon::ThreadPool;
use reqwest::blocking::Client;
use retry::{
    delay::{jitter, Exponential},
    retry_with_index,
};
use serde::Deserialize;
use serde_yaml::{self};
use std::time::Duration;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub proxies: Vec<ProxyConfig>,
}

#[derive(Debug, Deserialize)]
pub struct ProxyConfig {
    pub consumer_config: ConsumerConfig,
    pub http_config: HttpConfig,
    pub deadletter_config: Option<DeadLetterConfig>,
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
    pub target_host: String,
    pub target_endpoint: String,
    pub timeout: u64,
}

#[derive(Debug, Deserialize)]
pub struct DeadLetterConfig {
    pub host: String,
    pub topic: String,
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

pub struct ProxyApplication {
    proxies: Vec<Proxy>,
    worker_pool: ThreadPool,
}

impl ProxyApplication {
    pub fn build(config: Config) -> Result<ProxyApplication, &'static str> {
        let mut proxies: Vec<Proxy> = Vec::new();
        for proxy in config.proxies {
            let message_handler = MessageHandler::build(proxy.http_config, proxy.deadletter_config);
            let consumer = Proxy::build(proxy.consumer_config, message_handler).unwrap();
            proxies.push(consumer);
        }
        let worker_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(proxies.len())
            .build()
            .unwrap();

        return Ok(ProxyApplication {
            proxies,
            worker_pool,
        });
    }

    pub fn start(self) {
        self.worker_pool.scope(|s| {
            for mut proxy in self.proxies {
                s.spawn(move |_| proxy.start())
            }
        });
    }
}

pub struct Proxy {
    pub consumer: Consumer,
    pub worker_pool: ThreadPool,
    pub handler: MessageHandler,
}

impl Proxy {
    pub fn build(
        config: ConsumerConfig,
        message_handler: MessageHandler,
    ) -> Result<Proxy, &'static str> {
        let topic = String::from(config.topic);
        let consumer = Consumer::from_hosts(vec![config.host])
            .with_topic(topic.clone())
            .with_fallback_offset(FetchOffset::Latest)
            .with_group(format!("{}-{}", config.app_name.to_owned(), topic.clone()))
            .with_offset_storage(GroupOffsetStorage::Kafka)
            .create()
            .unwrap();

        let worker_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(config.pool_size.try_into().unwrap())
            .build()
            .unwrap();

        return Ok(Proxy {
            consumer,
            worker_pool,
            handler: message_handler,
        });
    }

    pub fn start(&mut self) {
        loop {
            let ms = self.consumer.poll().unwrap();
            if !ms.is_empty() {
                self.worker_pool.scope(|s| {
                    for set in ms.iter() {
                        for m in set.messages() {
                            let data = Vec::from(m.value);
                            s.spawn(|_| self.handler.handle(data));
                        }
                        let _ = self.consumer.consume_messageset(set);
                    }
                    self.consumer.commit_consumed().unwrap();
                })
            }
        }
    }
}

pub struct MessageHandler {
    client: Client,
    msg_destination: String,
    timeout_policy: Duration,
    dlq_config: Option<DeadLetterConfig>,
}

impl MessageHandler {
    pub fn build(http_config: HttpConfig, dlq_config: Option<DeadLetterConfig>) -> MessageHandler {
        MessageHandler {
            client: reqwest::blocking::Client::new(),
            msg_destination: format!(
                "{0}/{1}",
                http_config.target_host, http_config.target_endpoint
            ),
            timeout_policy: Duration::new(http_config.timeout, 0),
            dlq_config,
        }
    }

    pub fn handle(&self, data: Vec<u8>) {
        let result = retry_with_index(Exponential::from_millis(10).map(jitter).take(3), |x| {
            println!("retry attempt {}", x);
            let body = String::from_utf8(data.to_vec()).expect("expecting string body");
            println!("body: {}", body);
            let request = self.setup_http_request().body(body).send();
            if request.is_ok() {
                let resp = request.ok();
                return match resp {
                    Some(r) => match r.status().as_u16() {
                        s if s >= 200 && s < 400 => Ok(s),
                        s => Err(s),
                    },
                    None => Err(500),
                };
            }
            return Err(500);
        });

        match result {
            Ok(_) => println!("Message delivered"),
            Err(_) => self.handle_request_error(data),
        }
    }

    fn setup_http_request(&self) -> reqwest::blocking::RequestBuilder {
        self.client
            .post(self.msg_destination.clone())
            .timeout(self.timeout_policy)
    }

    fn handle_request_error(&self, data: Vec<u8>) {
        match &self.dlq_config {
            Some(c) => {
                let mut dlq = Producer::from_hosts(vec![c.host.clone()])
                    .with_ack_timeout(Duration::from_secs(1))
                    .with_required_acks(RequiredAcks::One)
                    .create()
                    .unwrap();
                println!("dumping to dlq");
                dlq.send(&Record::from_value(c.topic.as_str(), data))
                    .unwrap();
            }
            None => println!("Not dumping to dlq"),
        }
    }
}
