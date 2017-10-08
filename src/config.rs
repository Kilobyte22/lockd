use config_parser as cfg;
use std::error::Error;
use std::fmt;
use std::fmt::Debug;
use std::result::Result;
use std::fs::File;

pub const DEFAULT: &'static str = include_str!("../default.cfg");

pub enum DefaultValue {
    On,
    Off,
    Remember
}

pub struct Config {
    pub lock_command: String,
    pub lock_params: Vec<String>,
    pub default_autolock: DefaultValue,
    pub default_suspend_on_lid: DefaultValue,
    pub pre_lock: Option<(String, Vec<String>)>,
    pub post_unlock: Option<(String, Vec<String>)>
}

impl Config {

    pub fn parse(config: String) -> Result<Config, ConfigError> {

        let mut ret = Config {
            lock_command: format!("i3lock"),
            lock_params: vec![format!("-c"), format!("000000"), format!("--nofork")],
            default_autolock: DefaultValue::On,
            default_suspend_on_lid: DefaultValue::On,
            pre_lock: None,
            post_unlock: None
        };

        let c = match cfg::parse_string(config) {
            Ok(x) => x,
            Err(x) => return Err(ConfigError::parse(x))
        };

        match c.matching("lock_cmd").next() {
            Some(cmd) => {
                if cmd.len() < 1 {
                    return Err(ConfigError::option("lock_cmd", "You have to specify a command and optionally parameters"));
                }
                ret.lock_command = cmd.get(0).to_string();
                ret.lock_params = Vec::with_capacity(cmd.len() - 1);
                for i in 1..cmd.len() {
                    println!("Getting option {} of {}", i, cmd.len());
                    ret.lock_params.push(cmd.get(i).to_string());
                }
            },
            None => {}
        }

        if let Some(command) = c.matching("command").next() {
            if let Some(pre_lock) = command.matching("pre_lock").next() {
                let cmd = pre_lock.get_opt(0).map(|o| o.to_string());
                let mut args = Vec::new();
                for i in 1..pre_lock.len() {
                    args.push(pre_lock.get(i).to_string());
                };
                if let Some(cmd) = cmd {
                    ret.pre_lock = Some((cmd, args));
                }
            }
            if let Some(post_unlock) = command.matching("post_unlock").next() {
                let cmd = post_unlock.get_opt(0).map(|o| o.to_string());
                let mut args = Vec::new();
                for i in 1..post_unlock.len() {
                    args.push(post_unlock.get(i).to_string());
                };
                if let Some(cmd) = cmd {
                    ret.post_unlock = Some((cmd, args));
                }
            }
        }

        match c.matching("default").next() {
            Some(default) => {
                if let Some(autolock) = default.matching("autolock").next() {
                    ret.default_autolock = match autolock.get_opt(0) {
                        Some("on") => DefaultValue::On,
                        Some("off") => DefaultValue::Off,
                        Some("remember") => DefaultValue::Remember,
                        _ => DefaultValue::On
                    }
                }

                if let Some(lidaction) = default.matching("lidaction").next()  {
                    ret.default_suspend_on_lid = match lidaction.get_opt(0) {
                        Some("suspend") => DefaultValue::On,
                        Some("ignore") => DefaultValue::Off,
                        Some("remember") => DefaultValue::Remember,
                        _ => DefaultValue::On
                    }
                }
            },
            None => {}
        }

        Ok(ret)
    }

    pub fn get_lock_command(&self) -> (&str, &[String]) {
        (&self.lock_command, &self.lock_params)
    }
}

#[derive(Debug)]
pub enum ErrorType {
    ParseError(cfg::ParseError),
    OptionError(String, String)
}

#[derive(Debug)]
pub struct ConfigError {
    error_type: ErrorType,
    description: String
}

impl ConfigError {
    fn new(error_type: ErrorType) -> ConfigError {
        let desc = match &error_type {
            &ErrorType::ParseError(ref error) => {
                format!("Error while parsing: {:?}", error)
            },
            &ErrorType::OptionError(ref option, ref error) => {
                format!("Error in option {}: {}", option, error)
            }
        };

        ConfigError {
            error_type: error_type,
            description: desc
        }
    }

    fn parse(err: cfg::ParseError) -> ConfigError {
        ConfigError::new(ErrorType::ParseError(err))
    }

    fn option(option: &str, error: &str) -> ConfigError {
        ConfigError::new(ErrorType::OptionError(option.to_string(), error.to_string()))
    }
}

impl Error for ConfigError {
    fn description(&self) -> &str {
        &self.description
    }
 
    fn cause(&self) -> Option<&Error> {
        match &self.error_type {
            &ErrorType::ParseError(ref error) => None, //Some(&error),
            &ErrorType::OptionError(..) => None
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self.error_type {
            &ErrorType::ParseError(ref error) => { 
                try!(write!(f, "Parse Error: "));
                error.fmt(f)
            },
            &ErrorType::OptionError(..) =>
                write!(f, "{}", self.description())
        }
    }
}
