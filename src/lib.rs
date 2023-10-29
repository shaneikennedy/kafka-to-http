use serde::Deserialize;
use serde_yaml::{self};

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
