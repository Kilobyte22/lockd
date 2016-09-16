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
        let r = try!(call(&c, method("GetSuspendOnLid")));
        let lidaction: bool = r.get1().unwrap();
        match a {
            "lock" => try!(basic_call(&c, method("Lock"))),
            "unlock" => try!(basic_call(&c, method("Unlock"))),
            "exit" => try!(basic_call(&c, method("Exit"))),
            "lidaction" => if args.len() > 2 {
                let b: &str = &args[2];
                match b {
                    "suspend" => try!(basic_call(&c, method("SetSuspendOnLid").append1(true))),
                    "ignore" => try!(basic_call(&c, method("SetSuspendOnLid").append1(false))),
                    "toggle" => try!(basic_call(&c, method("SetSuspendOnLid").append1(!lidaction))),
                    _ => usage()
                }
            } else {
                if lidaction {
                    println!("suspend")
                } else {
                    println!("ignore")
                }
            },
            "autolock" => if args.len() > 2 {
                let b: &str = &args[2];
                match b {
                    "on" => try!(basic_call(&c, method("SetAutoLock").append1(true))),
                    "off" => try!(basic_call(&c, method("SetAutoLock").append1(false))),
                    _ => usage()
                }
            } else {
                let r = try!(call(&c, method("GetAutoLock")));
                if r.get1().unwrap() {
                    println!("on")
                } else {
                    println!("off")
                }
            },
            "perform_autolock" => try!(basic_call(&c, method("AutoLock"))),
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
lidaction [suspend|ignore|toggle] - gets or sets the lid action
autolock [on|off] - gets or sets the autolock state
perform_autolock - locks the screen if autolock is enabled
exit - exit the daemon cleanly"#;
    println!("Usage {} <command> [args...]", env::args().next().unwrap());
    println!("{}", usage);
}
