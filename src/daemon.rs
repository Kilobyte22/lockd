extern crate libc;
extern crate config_parser;
extern crate dbus;

use std::sync::mpsc;
use std::sync::mpsc::{Sender, Receiver};
use std::{thread, fs, path, env, process};
use std::io::{Result as IOResult, Write, Read};
use std::error::Error;

//mod config;
mod msg;
mod lockscreen;
mod inhibit;
mod react;
mod api;
mod config;

use msg::{LockMessage, InhibitMessage, CoreMessage, CoreFlag};

struct ActorMainHandles {
    lockscreen: Sender<LockMessage>,
    inhibitors: Sender<InhibitMessage>,
    core: Sender<CoreMessage>
}

struct State {
    locked: bool,
    inhibit_lid: bool,
    locking: bool,
    should_exit: bool,
    autolock: bool,
    pre_lock: Option<(String, Vec<String>)>,
    post_unlock: Option<(String, Vec<String>)>
}

impl Default for State {
    fn default() -> State {
        State {
            locked: false,
            inhibit_lid: false,
            locking: false,
            should_exit: false,
            autolock: true,
            pre_lock: None,
            post_unlock: None
        }
    }
}

impl State {
    fn write<W: Write>(&self, w: &mut W) -> IOResult<()> {
        write!(w, "autolock {:?};\n", self.autolock)?;
        write!(w, "inhibit_lid {:?};\n", self.inhibit_lid)?;        
        Ok(())
    }

    fn read(p: &path::Path) -> IOResult<State> {
        if p.exists() {
            match config_parser::parse_file(fs::File::open(p)?) {
                Ok(loaded) => {
                    let mut state = State::default();
                    if let Some(autolock) = loaded.matching("autolock").next() {
                        if let Some(value) = autolock.get_opt(0) {
                            if let Ok(value) = value.parse() {
                                state.autolock = value;
                            }
                        }
                    }
                    if let Some(inhibit_lid) = loaded.matching("inhibit_lid").next() {
                        if let Some(value) = inhibit_lid.get_opt(0) {
                            if let Ok(value) = value.parse() {
                                state.inhibit_lid = value;
                            }
                        }
                    }
                    Ok(state)
                },
                Err(e) => {
                    eprintln!("Could not load state: {:?}", e);
                    Ok(State::default())
                }
            }
        } else {
            Ok(State::default())
        }
    }

    fn load(&mut self, p: &path::Path, c: &config::Config) -> IOResult<()> {
        let loaded_state = State::read(p)?;
        self.autolock = match c.default_autolock {
            config::DefaultValue::Remember => {
                loaded_state.autolock
            },
            config::DefaultValue::On => true,
            config::DefaultValue::Off => false
        };

        self.inhibit_lid = match c.default_suspend_on_lid {
            config::DefaultValue::Remember => {
                 loaded_state.inhibit_lid
            },
            config::DefaultValue::On => false,
            config::DefaultValue::Off => true
        };
        
        Ok(())
    }
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
        inhibitors: inh_send,
        core: core_send
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

fn state_path() -> String {
    let file = match env::var("HOME") {
        Ok(home) => {
            format!("{}/.local/share/lockd/state", home)
        },
        Err(e) => {
            println!("Warning: Could not get $HOME: {}, defaulting to config file /tmp/lockd_state", e);
            "/tmp/lockd_state".to_string()
        }
    };

    {
        let path = path::Path::new(&file);

        match fs::metadata(path) {
            Ok(_) => (),
            // File does not exist or we lack permission.
            Err(_) => {
                create_path(path.parent().expect("Uhh config file in root? wat.")).expect("Could not create directory for state");
            }
        };
    }

    file
}

fn load_config() -> Option<config::Config> {
    #[allow(unused_assignments)]
    let mut pathstr = String::new();
    let path = match env::var("HOME") {
        Ok(home) => {
            pathstr = format!("{}/.config/lockd/main.cfg", home);
            path::Path::new(&pathstr)
        },
        Err(e) => {
            println!("Warning: Could not get $HOME: {}, defaulting to config file /etc/lockd.cfg", e);
            path::Path::new("/etc/lockd.cfg")
        }
    };

    let data = match fs::metadata(path) {
        Ok(md) => Some(md),
        // File does not exist or we lack permission.
        Err(_) => {
            match create_path(path.parent().expect("Uhh config file in root? wat.")).and_then(|_| write_file(path, config::DEFAULT)) {
                Ok(_) => {
                    // We can panic here, i just created the file so it should exist and we should
                    // have permissions for it
                    Some(fs::metadata(path).expect(""))
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
            f.read_to_string(&mut s).unwrap();
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

fn apply_config(config: &config::Config, handles: &ActorMainHandles, state: &mut State) {
    let cmd = config.get_lock_command();
    let lcmd = cmd.0;
    let lparam = cmd.1.iter().map(|s| s.to_string()).collect();
    handles.lockscreen.send(LockMessage::SetLockscreen(lcmd.to_string(), lparam)).unwrap();
    state.pre_lock = config.pre_lock.clone();
    state.post_unlock = config.post_unlock.clone();
}

fn run_command(cmd: &Option<(String, Vec<String>)>) {
    if let &Some(ref cmd) = cmd {
        match process::Command::new(&cmd.0).args(&cmd.1).spawn() {
            Ok(_) => (),
            Err(e) => println!("Failed to start {}: {}", cmd.0, e)
        }
    }
}

fn actor_main(handles: ActorMainHandles, inbox: Receiver<CoreMessage>) {
    let mut state = State {
            locked: false,
            inhibit_lid: false,
            locking: false,
            should_exit: false,
            autolock: true,
            pre_lock: None,
            post_unlock: None
    };
    handles.inhibitors.send(InhibitMessage::CreateDelay).unwrap();
    let state_path = state_path();
    {
        let cfg = load_config().expect("Could not load configuration");
        apply_config(&cfg, &handles, &mut state);
        if let Err(e) = state.load(path::Path::new(&state_path), &cfg) {
            println!("Failed to load state: {}", e.description());
        }
    }
    for message in inbox {
        println!("Received message in core: {:?}", message);
        match message {
            CoreMessage::Lock => 
                if !(state.locked || state.locking) {
                    run_command(&state.pre_lock);
                    handles.lockscreen.send(LockMessage::Lock).unwrap();
                    state.locking = true;
                },
            CoreMessage::Unlock => 
                if state.locked && !state.locking {
                    state.locking = true;
                    handles.lockscreen.send(LockMessage::Unlock).unwrap();
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
                run_command(&state.post_unlock);
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
                state.write(&mut fs::File::create(&state_path).unwrap()).unwrap();
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
                    handles.core.send(CoreMessage::Lock).unwrap();
                }
            },
            CoreMessage::SetAutoLock(value) => {
                state.autolock = value;
                state.write(&mut fs::File::create(&state_path).unwrap()).unwrap();
            },
            CoreMessage::ReloadConfig => {
                let config = match load_config() {
                    Some(c) => c,
                    None => {
                        println!("Warning: could not load configuration file");
                        continue;
                    }
                };
                apply_config(&config, &handles, &mut state);
            },
            CoreMessage::LockCrashed => {
                if state.locked {
                    if state.locking {
                        // We are currently unlocking, so this is actually what we want.
                        // Lets update our own state
                        handles.core.send(CoreMessage::Unlocked).unwrap();
                    } else {
                        state.locking = true;
                        state.locked = false;
                        handles.lockscreen.send(LockMessage::Lock).unwrap();
                    }
                }
            }

        }
    }
}
