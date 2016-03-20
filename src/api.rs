use std::sync::mpsc::Sender;
use std::sync::mpsc;
use msg::{CoreMessage, CoreFlag};
use dbus::{Connection, BusType, NameFlag};
use dbus::tree::Factory;

pub fn actor_api(core: Sender<CoreMessage>) {
  let c = Connection::get_private(BusType::Session).unwrap();
  c.register_name("de.kilobyte22.lockd", NameFlag::ReplaceExisting as u32).unwrap();
  let f = Factory::new_fn();

  let tree = f.tree().add(f.object_path("/de/kilobyte22/lockd")
    .introspectable().add(
      f.interface("de.kilobyte22.lockd.Control").add_m(
        f.method("Lock", |m, _, _| {
          core.send(CoreMessage::Lock).unwrap();
          Ok(vec![m.method_return()])
        })
      ).add_m(
        f.method("Unlock", |m, _, _| {
          core.send(CoreMessage::Unlock).unwrap();
          Ok(vec![m.method_return()])
        })
      ).add_m(
        f.method("SetSuspendOnLid", |m, _, _| {
          core.send(CoreMessage::SuspendOnLid(m.get1().unwrap())).unwrap();
          Ok(vec![m.method_return()])
        }).inarg::<bool, _>("value")
      ).add_m(
        f.method("GetSuspendOnLid", |m, _, _| {
          let (tx, rx) = mpsc::channel::<bool>();
          core.send(CoreMessage::QueryFlag(CoreFlag::SuspendOnLid, tx)).unwrap();
          Ok(vec![m.method_return().append1(rx.recv().unwrap())])
        }).outarg::<bool, _>("value")
      ).add_m(
        f.method("Exit", |m, _, _| {
          core.send(CoreMessage::Exit).unwrap();
          Ok(vec![m.method_return()])
        })
      )
    )
  );

  tree.set_registered(&c, true).unwrap();
  for _ in tree.run(&c, c.iter(1000)) {}
}
