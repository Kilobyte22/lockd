extern crate dbus;
extern crate core;

use std::env;
use core::result::Result;
use dbus::{BusType, Connection, Message, Error};

fn main() {
  match exec() {
    Ok(_) => {},
    Err(e) => {
      println!("DBus Error: {}", e);
      println!("Is the daemon running?");
    }
  }
}

fn exec() -> Result<(), Error> {
  let args: Vec<_> = env::args().collect();

  let c = Connection::get_private(BusType::Session).unwrap();

  if args.len() > 1 {
    let a: &str = &args[1];
    match a {
      "lock" => try!(basic_call(&c, method("Lock"))),
      "unlock" => try!(basic_call(&c, method("Unlock"))),
      "exit" => try!(basic_call(&c, method("Exit"))),
      "lidaction" => if args.len() > 2 {
        let b: &str = &args[2];
        match b {
          "suspend" => try!(basic_call(&c, method("SetSuspendOnLid").append1(true))),
          "ignore" => try!(basic_call(&c, method("SetSuspendOnLid").append1(false))),
          _ => usage()
        }
      } else {
        let r = try!(call(&c, method("GetSuspendOnLid")));
        if r.get1().unwrap() {
          println!("suspend")
        } else {
          println!("ignore")
        }
      },
      _ => usage()
    };
  } else {
    usage();
  }
  Ok(())
}

fn basic_call(con: &Connection, m: Message) -> Result<(), Error> {
  match call(con, m) {
    Ok(_) => Ok(()),
    Err(x) => Err(x)
  }
}

fn call(con: &Connection, m: Message) -> Result<Message, Error> {
  con.send_with_reply_and_block(m, 2000)
}

fn method(name: &str) -> Message {
  Message::new_method_call(
      "de.kilobyte22.lockd",
      "/de/kilobyte22/lockd",
      "de.kilobyte22.lockd.Control",
      name
  ).unwrap()
}

fn usage() {
  let usage = r#"
Commands:
  
lock - instantly locks the screen
unlock - instantly unlocks the screen
lidaction [suspend|ignore] - gets or sets the lid action
exit - exit the daemon cleanly"#;
  println!("Usage {} <command> [args...]", env::args().next().unwrap());
  println!("{}", usage);
}
