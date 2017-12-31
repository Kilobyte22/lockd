extern crate libc;
extern crate config_parser;
extern crate dbus;

use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::{thread, fs, path, env};
use std::io::{Result as IOResult, Write, Read};

//mod config;
mod msg;
mod lockscreen;
mod inhibit;
mod react;
mod api;
mod config;

macro_rules! dbgprintln {
    ($fmt:expr) => (if cfg!(debug){println!($fmt)});
    ($fmt:expr, $($arg:tt)*) => (if cfg!(debug){println!($fmt, $($arg)*)});
}

use msg::{LockMessage, InhibitMessage, CoreMessage, CoreFlag};

struct ActorMainHandles {
    lockscreen: Sender<LockMessage>,
    inhibitors: Sender<InhibitMessage>
}

struct State {
    locked: bool,
    inhibit_lid: bool,
    locking: bool,
    should_exit: bool,
    autolock: bool
}

fn main() {
    let (core_send, core_recv) = mpsc::channel();
    let (inh_send, inh_recv) = mpsc::channel();
    let (lock_send, lock_recv) = mpsc::channel();

    let core = core_send.clone();
    thread::spawn(||{
        lockscreen::actor_lockscreen(core, lock_recv);
    });
    let core = core_send.clone();
    thread::spawn(||{
        inhibit::actor_inhibit(core, inh_recv);
    });
    let core = core_send.clone();
    thread::spawn(||{
        react::actor_react(core);
    });
    let core = core_send.clone();
    thread::spawn(||{
        api::actor_api(core);
    });

    let handles = ActorMainHandles {
        lockscreen: lock_send,
        inhibitors: inh_send
    };

    actor_main(handles, core_recv);
}

fn create_path(path: &path::Path) -> IOResult<()> {
    // This function is recursive.
    match fs::metadata(path) {
        Ok(md) => {
            // path exists, now to check if it actually is a dir
            if md.is_dir() {
                return Ok(());
            }
            // path exists but is no actual dir.
            // I will now intentionally cause an io error.
            fs::create_dir(path)
        },
        Err(_) => {
            // Dir does not actually exist. Lets ensure its parent exists first
            match path.parent() {
                Some(p) => try!(create_path(p)),
                None => ()
            };
            // And create the dir itself
            fs::create_dir(path)
        }
    }
}

fn write_file(path: &path::Path, content: &str) -> IOResult<()> {
    let mut f = try!(fs::File::create(path));
    try!(f.write_all(content.as_bytes()));
    Ok(())
}

fn load_config() -> Option<config::Config> {
    let mut pathstr = String::new();
    let path = match env::var("XDG_CONFIG_HOME").or(env::var("HOME")) {
        Some(path) => {
            pathstr = format!("{}/lockd/main.cfg");
            path::Path::new(&pathstr);
        },
        Err(e) => {
            panic!(format!("Your system is broken and does not have a working $HOME. PLZ FIX. Additional details: {}", e))
        }
    };
    /*
    let path = match env::var("HOME") {
        Ok(home) => {
            pathstr = format!("{}/.config/lockd/main.cfg", home);
            path::Path::new(&pathstr)
        },
        Err(e) => {
            println!("Warning: Could not get $HOME: {}, defaulting to config file /etc/lockd.cfg", e);
            path::Path::new("/etc/lockd.cfg")
        }
    };*/

    let data = match fs::metadata(path) {
        Ok(md) => Some(md),
        // File does not exist or we lack permission.
        Err(_) => {
            match create_path(path.parent().expect("Uhh config file in root? wat.")).and_then(|_| write_file(path, config::DEFAULT)) {
                Ok(_) => {
                    // We can panic here, i just created the file so it should exist and we should
                    // have permissions for it
                    Some(fs::metadata(path).expect("OSI LAYER 8 ERROR DETECTED IN PROGRAMMER BRAIN. CAN'T WORK UNDER THESE CIRCUMSTANCES."))
                },
                // Apparently we don't actually have permision to write the file
                Err(e) => {
                    println!("Warning: Error while writing initial configuration file at {}: {}", path.to_str().unwrap(), e);
                    None
                }
            }
        }
    };

    // So now we should have a config file given we have metadata
    match data {
        Some(_) => {
            // TODO: Proper error handling so we can recover
            let mut f = fs::File::open(path).expect("Could not open config file");
            let mut s = String::new();
            f.read_to_string(&mut s);
            let cfg = config::Config::parse(s).unwrap();
            Some(cfg)
        },
        None => {
            println!("Warning: taking default config but not writing it");
            Some(config::Config::parse(config::DEFAULT.to_string()).expect("default config to parse"))
        }
    }

    // Now that we have the file path we check if it actually exists
    // TODO: Properly handle the corner case that the user lacks permissions to access the path

}

fn apply_config(config: config::Config, handles: &ActorMainHandles) {
    let cmd = config.get_lock_command();
    let lcmd = cmd.0;
    let lparam = cmd.1.iter().map(|s| s.to_string()).collect();
    handles.lockscreen.send(LockMessage::SetLockscreen(lcmd.to_string(), lparam));
}

fn actor_main(handles: ActorMainHandles, inbox: Receiver<CoreMessage>) {
    let mut state = State {
            locked: false,
            inhibit_lid: false,
            locking: false,
            should_exit: false,
            autolock: true
    };
    handles.inhibitors.send(InhibitMessage::CreateDelay).unwrap();
    {
        let cfg = load_config();
        apply_config(cfg.expect("Could not load configuration"), &handles);
    }
    for message in inbox {
        println!("Received message in core: {:?}", message);
        match message {
            CoreMessage::Lock => 
                if !(state.locked || state.locking) {
                    handles.lockscreen.send(LockMessage::Lock).unwrap();
                    state.locking = true;
                },
            CoreMessage::Unlock => 
                if state.locked && !state.locking {
                    handles.lockscreen.send(LockMessage::Unlock).unwrap();
                    state.locking = true;
                },
            CoreMessage::Locked => {
                state.locked = true;
                state.locking = false;
                handles.inhibitors.send(InhibitMessage::ReleaseDelay).unwrap();
            },
            CoreMessage::Unlocked => {
                state.locked = false;
                state.locking = false;
                if state.should_exit {
                        std::process::exit(0);
                }
                handles.inhibitors.send(InhibitMessage::CreateDelay).unwrap();
            },
            CoreMessage::Exit => {
                if state.locked {
                    handles.lockscreen.send(LockMessage::Unlock).unwrap();
                } else {
                    std::process::exit(0);
                }
                state.should_exit = true;
            },
            CoreMessage::SuspendOnLid(value) => {
                // inhibit_lid and suspend_on_lid are opposite things
                // hence we need a == here and not a !=
                if value == state.inhibit_lid {
                    state.inhibit_lid = !value;
                    if state.inhibit_lid {
                            handles.inhibitors.send(InhibitMessage::CreateBlock).unwrap();
                    } else {
                            handles.inhibitors.send(InhibitMessage::ReleaseBlock).unwrap();
                    }
                }
            },
            CoreMessage::Suspending => {
                if !state.locked {
                    handles.lockscreen.send(LockMessage::Lock).unwrap();
                    state.locking = true;
                }
            },
            CoreMessage::Suspended => {
                if !state.locked {
                    handles.lockscreen.send(LockMessage::Lock).unwrap();
                    state.locking = true;
                }
            },
            CoreMessage::QueryFlag(flag, channel) => {
                channel.send(match flag {
                    CoreFlag::SuspendOnLid => !state.inhibit_lid,
                    //CoreFlag::Locking => state.locking,
                    //CoreFlag::Locked => state.locked,
                    CoreFlag::AutoLock => state.autolock
                }).unwrap();
            },
            CoreMessage::AutoLock => {
                if !state.locked && !state.locking && state.autolock {
                    handles.lockscreen.send(LockMessage::Lock).unwrap();
                }
            },
            CoreMessage::SetAutoLock(value) => {
                state.autolock = value;
            },
            CoreMessage::ReloadConfig => {
                let config = match load_config() {
                    Some(c) => c,
                    None => {
                        println!("Warning: could not load configuration file");
                        continue;
                    }
                };
                apply_config(config, &handles);
            }

        }
    }
}
