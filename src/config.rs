use std::fmt;
use url::Url;

#[derive(Debug, Clone)]
pub struct Config {
    pub base_url: Url,
}

#[derive(Debug, Clone)]
pub enum Environment {
    Local,
    Production,
    Custom(String),
}

impl fmt::Display for Environment {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Environment::Local => write!(f, "local"),
            Environment::Production => write!(f, "production"),
            Environment::Custom(_) => write!(f, "custom"),
        }
    }
}

impl Config {
    pub fn new(env: &Environment) -> anyhow::Result<Self> {
        let base_url = match env {
            Environment::Local => Url::parse("http://localhost:3001")?,
            Environment::Production => Url::parse("https://bench.chat")?,
            Environment::Custom(url) => Url::parse(url)?,
        };
        Ok(Self { base_url })
    }

    pub fn ws_url_endpoint(&self) -> String {
        format!("{}api/terminal/ws-url", self.base_url)
    }

    pub fn auth_url(&self, token: &str) -> String {
        format!("{}auth/{}", self.base_url, token)
    }
}
