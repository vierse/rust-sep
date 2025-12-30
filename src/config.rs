use anyhow::{Result, anyhow, bail};
use config::{Config, File};
use serde::Deserialize;
use url::Url;

const DEFAULT_CONFIG_PATH: &str = "settings.yml";
const APP_PORT_ENV: &str = "APP_PORT";
const DATABASE_URL_ENV: &str = "DATABASE_URL";

pub struct Settings {
    pub port: u16,
    pub database_url: Url,
}

#[derive(Deserialize)]
struct DefaultConfig {
    app_port: u16,
    db_name: String,
    db_host: String,
    db_port: u16,
    db_user: String,
    db_pass: String,
}

fn load_default_config() -> Result<DefaultConfig> {
    let settings = Config::builder()
        .add_source(File::with_name(DEFAULT_CONFIG_PATH))
        .build()
        .map_err(|_| anyhow!("Failed to read config file"))?;

    settings
        .try_deserialize::<DefaultConfig>()
        .map_err(|_| anyhow!("Failed to deserialize config file"))
}

/// Try to parse env variable. If it's not set, return None. If it's invalid, treat it as an error.
fn try_from_env<T, F>(env_var: &str, f: F) -> Result<Option<T>>
where
    F: FnOnce(String) -> Result<T>,
{
    match std::env::var(env_var) {
        Ok(raw) => {
            let val = f(raw).map_err(|_| anyhow!("Failed to parse {}", env_var))?;
            Ok(Some(val))
        }
        Err(std::env::VarError::NotPresent) => Ok(None),
        Err(_) => bail!("Could not read {env_var} from env"),
    }
}

/// Load configuration from env with fallback to default config file. Early returns if everything is set in env.
pub fn load() -> Result<Settings> {
    let port_opt: Option<u16> = try_from_env(APP_PORT_ENV, |env_str| {
        env_str.parse::<u16>().map_err(|e| e.into())
    })?;

    let database_url_opt: Option<Url> = try_from_env(DATABASE_URL_ENV, |env_str| {
        Url::parse(&env_str).map_err(|e| e.into())
    })?;

    // to avoid destructuring database_url_opt (we need it later)
    #[allow(clippy::unnecessary_unwrap)]
    if port_opt.is_some() && database_url_opt.is_some() {
        return Ok(Settings {
            port: port_opt.unwrap(),
            database_url: database_url_opt.unwrap(),
        });
    }

    let config = load_default_config()?;

    let port = match port_opt {
        Some(val) => val,
        None => {
            tracing::warn!("{APP_PORT_ENV} is not set, using value from {DEFAULT_CONFIG_PATH}");
            config.app_port
        }
    };

    let database_url = match database_url_opt {
        Some(url) => url,
        None => {
            tracing::warn!("{DATABASE_URL_ENV} is not set, using value from {DEFAULT_CONFIG_PATH}");
            let url_str = format!(
                "postgres://{}:{}@{}:{}/{}",
                config.db_user, config.db_pass, config.db_host, config.db_port, config.db_name
            );
            Url::parse(&url_str)?
        }
    };

    Ok(Settings { port, database_url })
}
