use bawa::{app, cli, ui};
use clap_complete::CompleteEnv;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    CompleteEnv::with_factory(cli::build_command).complete();

    let mut app = app::App::new()?;

    if cli::handle_subcommands(&mut app) {
        return Ok(());
    }

    let res = app.run().await;
    ui::restore();

    if let Err(e) = res {
        eprintln!("{e:?}");
    }

    Ok(())
}
