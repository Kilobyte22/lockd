use clap::{Arg, Command};
use lockd_common::dbus::lockd_manager::ManagerProxy as LockdProxy;
use lockd_common::dbus::logind_manager::ManagerProxy as LogindProxy;
use zbus::Connection;

fn args() -> clap::Command {
    clap::command!()
        .subcommand_required(true)
        .subcommand(Command::new("lock").arg(Arg::new("ID").id("ID").required(true)))
        .subcommand(Command::new("reload"))
}

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let args = args().get_matches();

    let session = Connection::session().await?;
    let lockd = LockdProxy::new(&session).await?;

    match args.subcommand().expect("subcommand must be specified") {
        ("lock", args) => {
            lockd.lock(&args.get_one::<String>("ID").unwrap()).await?;
        }
        ("reload", args) => {
            lockd.reload().await?;
        }
        (_, _) => unreachable!(),
    }

    Ok(())
}
