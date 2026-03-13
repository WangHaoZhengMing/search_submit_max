use anyhow::Context;
use config::{Config, FileFormat};
use serde::Deserialize;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::LazyLock;

pub static CONFIG: LazyLock<AppConfig> =
    LazyLock::new(|| AppConfig::load().expect("Failed to initialize config"));

static CHECK_INDEX: AtomicUsize = AtomicUsize::new(0);

#[allow(dead_code)]
#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub haoran_cookie: String,
    pub cookie_bai: Option<String>,
    pub cookie_zhang: Option<String>,
    pub cookie_xin: Option<String>,
    pub cookie_baiqian: Option<String>,
    #[serde(skip)]
    pub valid_cookies: Vec<String>,
    pub tikutoken: String,
    pub toml_folder: String,
    pub llm_api_key: String,
    pub llm_api_base_url: String,
    pub llm_model_name: String,
    #[serde(default = "default_paper_concurrency")]
    pub paper_concurrency: usize,
    #[serde(default = "default_question_concurrency")]
    pub question_concurrency: usize,
    #[serde(default = "default_search_max_retries")]
    pub search_max_retries: usize,
}

fn default_paper_concurrency() -> usize {
    30
}

fn default_question_concurrency() -> usize {
    50
}

fn default_search_max_retries() -> usize {
    3
}

impl AppConfig {
    pub fn load() -> anyhow::Result<Self> {
        let mut config: AppConfig = Config::builder()
            .add_source(
                config::File::with_name("application")
                    .format(FileFormat::Yaml)
                    .required(true),
            )
            .add_source(
                config::Environment::with_prefix("APP")
                    .try_parsing(true)
                    .separator("_")
                    .list_separator(","),
            )
            .build()
            .with_context(|| anyhow::anyhow!("Failed to load config"))?
            .try_deserialize()
            .with_context(|| anyhow::anyhow!("Failed to deserialize config"))?;

        config.valid_cookies.push(config.haoran_cookie.clone());
        if let Some(c) = &config.cookie_bai {
            if !c.trim().is_empty() {
                config.valid_cookies.push(c.clone());
            }
        }
        if let Some(c) = &config.cookie_zhang {
            if !c.trim().is_empty() {
                config.valid_cookies.push(c.clone());
            }
        }
        if let Some(c) = &config.cookie_xin {
            if !c.trim().is_empty() {
                config.valid_cookies.push(c.clone());
            }
        }
        if let Some(c) = &config.cookie_baiqian {
            if !c.trim().is_empty() {
                config.valid_cookies.push(c.clone());
            }
        }
        Ok(config)
    }
}

pub fn get() -> &'static AppConfig {
    &CONFIG
}

pub fn get_cookie() -> &'static str {
    let cookies = &CONFIG.valid_cookies;
    if cookies.is_empty() {
        return &CONFIG.haoran_cookie;
    }
    let index = CHECK_INDEX.fetch_add(1, Ordering::SeqCst);
    &cookies[index % cookies.len()]
}

pub fn get_haoran_cookie() -> &'static str {
    &CONFIG.haoran_cookie
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_load_config() {
        let config = AppConfig::load().expect("Failed to load config");
        println!("{:#?}", config);
    }
}
