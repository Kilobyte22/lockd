use toml::{Parser, Value, ParserError};
use std::error::Error;
use std::fmt;
use std::result::Result;

struct Config {
    lock_command: String
}

impl Config {

  fn parse(config: String) -> Result<Config, ConfigError> {
    let mut parser = Parser::new(&config);
    match parser.parse() {
      Some(data) => {

        Ok(Config { lock_command: "i3lock".to_string() })
      },
      None => ConfigError::ParseError(parser.errors.first().unwrap())
    }
  }

  fn get_lock_command(&self) -> String {
    return self.lock_command.clone();
  }
}

#[derive(Debug)]
enum ConfigError {
  ParseError(ParserError),
  OptionError(String, String)
}

impl Error for ConfigError {
  fn description(&self) -> &str {
    match *self {
      ConfigError::ParseError(error) => format!("Error while parsing: {}", error.description),
      ConfigError::OptionError(option, error) => format!("Error in option {}: {}", option, error)
    }
  }
 
  fn cause(&self) -> Option<&Error> {
    match *self {
      ConfigError::ParseError(error) => Some(&error),
      ConfigError::OptionError(..) => None
    }
  }
}

impl fmt::Display for ConfigError {
  fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
    match *self {
      ConfigError::ParseError(error) => { 
        try!(write!(f, "Parse Error: "));
        error.fmt(f)
      },
      ConfigError::OptionError(..) =>
        write!(f, "{}", self.description())
    }
  }
}
