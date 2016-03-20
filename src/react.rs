use dbus::{Connection, BusType, ConnectionItem};
use std::sync::mpsc::{Sender};
use msg::CoreMessage;

pub fn actor_react(core: Sender<CoreMessage>) {
  let con = Connection::get_private(BusType::System).unwrap();
  con.add_match("type='signal',interface='org.freedesktop.login1.Manager'").unwrap();
  for event in con.iter(60_000) {
    match event {
      ConnectionItem::Signal(msg) => {
        let member = msg.member().unwrap();
        if &*member == "PrepareForSleep" {
          let active: bool = msg.get1().unwrap();
          if active {
            core.send(CoreMessage::Suspending).unwrap();
          } else {
            core.send(CoreMessage::Suspended).unwrap();
          }
        } else {
        }
      },
      ConnectionItem::MethodCall(..) => panic!("Method call on connection"),
      ConnectionItem::MethodReturn(..) => panic!("Weird Method Return"),
      ConnectionItem::Nothing => {},
      ConnectionItem::WatchFd(..) => {}
    }
  }
}
