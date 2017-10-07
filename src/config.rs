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
    lock_command: String,
    lock_params: Vec<String>,
    default_autolock: DefaultValue,
    default_suspend_on_lid: DefaultValue
}

impl Config {

    pub fn parse(config: String) -> Result<Config, ConfigError> {

        let mut ret = Config {
            lock_command: format!("i3lock"),
            lock_params: vec![format!("-c"), format!("000000"), format!("--nofork")],
            default_autolock: On,
            default_suspend_on_lid: On
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

        match c.matching("default").next() {
            Some(default) => {
                match default.matching("autolock").next() {
                    Some(autolock) => {
                        ret.autolock = match autolock.get_opt(0) {
                            Some("on") => DefaultValue::On,
                            Some("off") => DefaultValue::Off,
                            Some("remember") => DefaultValue::Remember,
                            _ => DefaultValue::On
                        }
                    },
                    None => {}
                }

                match default.matching("lidaction").next() {
                    Some(lidaction) => {
                        ret.default_suspend_on_lid = match lidaction.get_opt(0) {
                            Some("suspend") => DefaultValue::On,
                            Some("ignore") => DefaultValue::Off,
                            Some("remember") => DefaultValue::Remember
                        }
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
