use config::{Config as ConfigBuilder, ConfigError, Environment, File};
use core::fmt;
use serde::Deserialize;
use std::{fmt::Display, path::Path};

#[derive(Deserialize, Debug)]
pub struct Config {
    pub model_path: Box<Path>,
}

impl Config {
    pub fn from_config_file() -> Result<Self, ConfigError> {
        let builder = ConfigBuilder::builder();

        // 加载配置文件
        let builder = builder.add_source(File::with_name("config.toml"));

        // 添加环境变量配置
        let builder = builder.add_source(Environment::with_prefix("VAD").separator("__"));

        let builder = builder.build()?;

        builder.try_deserialize()
    }
}

impl Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "model path: {}", self.model_path.display())
    }
}
