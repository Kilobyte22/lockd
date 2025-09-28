use serde::Deserialize;
use std::collections::HashMap;
use std::ops::Deref;
use std::path::{Path, PathBuf};

pub struct ConfigBundle {
    pub config: Config,
    pub path: PathBuf,
}

impl ConfigBundle {
    pub async fn load(path: PathBuf) -> anyhow::Result<Self> {
        Ok(ConfigBundle {
            config: Config::load(&path).await?,
            path,
        })
    }

    pub async fn reload(&mut self) -> anyhow::Result<()> {
        self.config = Config::load(&self.path).await?;
        Ok(())
    }
}

impl Deref for ConfigBundle {
    type Target = Config;

    fn deref(&self) -> &Self::Target {
        &self.config
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub lockscreens: HashMap<String, Lockscreen>,
    #[serde(rename = "default-lockscreen")]
    pub default_lockscreen: String,
}

impl Config {
    pub async fn load<P: AsRef<Path>>(file: P) -> anyhow::Result<Self> {
        let contents = tokio::fs::read(file).await?;
        let config: Config = toml::from_slice(&contents)?;
        config.validate()?;
        Ok(config)
    }

    fn validate(&self) -> anyhow::Result<()> {
        let mut errors = Vec::new();
        for (name, lockscreen) in &self.lockscreens {
            if lockscreen.command.len() == 0 {
                errors.push(format!("Lockscreen {name} has empty command"));
            }
        }

        if !self.lockscreens.contains_key(&self.default_lockscreen) {
            errors.push(format!(
                "Default Lockscreen {} undefined",
                self.default_lockscreen
            ));
        }

        if !errors.is_empty() {
            anyhow::bail!("Config invalid: \n* {}", errors.join("* \n"));
        }
        Ok(())
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct Lockscreen {
    pub command: Vec<String>,
    #[serde(rename = "can-kill")]
    pub can_kill: bool,
}
