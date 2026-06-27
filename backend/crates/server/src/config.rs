use std::env;

use anyhow::{Context, Result};

#[derive(Debug, Clone)]
pub struct Settings {
    pub database_url: String,
    pub bind_addr: String,
    pub github_tokens: Vec<String>,
    pub admin_token: String,
    pub schedule_github_secs: u64,
    pub schedule_hn_secs: u64,
}

impl Settings {
    pub fn from_env() -> Result<Self> {
        let github_tokens: Vec<String> = env::var("GITHUB_TOKENS")
            .unwrap_or_default()
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();

        let parse_secs = |name: &str, default: &str| -> Result<u64> {
            env::var(name)
                .unwrap_or_else(|_| default.to_string())
                .parse::<u64>()
                .with_context(|| format!("invalid {name}"))
        };

        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost:5432/openradar".into()),
            bind_addr: env::var("BIND_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".into()),
            github_tokens,
            admin_token: env::var("ADMIN_TOKEN").unwrap_or_else(|_| "changeme".into()),
            schedule_github_secs: parse_secs("SCHEDULE_GITHUB_SECS", "3600")?,
            schedule_hn_secs: parse_secs("SCHEDULE_HN_SECS", "1800")?,
        })
    }
}
