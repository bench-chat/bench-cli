use std::fs::{self, File};
use std::io::{self, Read};
use std::path::PathBuf;
use rand::Rng;
use tracing::debug;

pub struct TokenManager {
    file_path: PathBuf,
}

impl TokenManager {
    pub fn new() -> io::Result<Self> {
        let home_dir = dirs::home_dir().ok_or_else(|| {
            io::Error::new(io::ErrorKind::NotFound, "Could not find home directory")
        })?;
        
        Ok(Self {
            file_path: home_dir.join(".bench.env"),
        })
    }

    pub fn load_token(&self) -> io::Result<Option<String>> {
        if !self.file_path.exists() {
            return Ok(None);
        }

        let mut content = String::new();
        File::open(&self.file_path)?.read_to_string(&mut content)?;

        for line in content.lines() {
            if let Some(token) = line.strip_prefix("BENCH_TOKEN=") {
                debug!("Loaded existing token from config file");
                return Ok(Some(token.to_string()));
            }
        }

        Ok(None)
    }

    pub fn save_token(&self, token: &str) -> io::Result<()> {
        let content = format!("BENCH_TOKEN={}\n", token);
        fs::write(&self.file_path, content)?;
        debug!("Saved token to config file");
        Ok(())
    }

    pub fn generate_token() -> String {
        const CHARSET: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
        let mut rng = rand::rng();
        
        (0..32)
            .map(|_| {
                let idx = rng.random_range(0..CHARSET.len());
                CHARSET[idx] as char
            })
            .collect()
    }
}
