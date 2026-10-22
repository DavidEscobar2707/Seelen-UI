mod debugger;
mod self_pipe;
pub mod shortcuts;
mod svc_pipe;

pub use self_pipe::SelfPipe;
pub use svc_pipe::ServicePipe;

use std::sync::atomic::Ordering;

use slu_ipc::commands::AppCommand;

use crate::{
    error::Result,
    resources::cli as resources_cli,
    virtual_desktops::cli as vd_cli,
    widgets::{
        cli as widget_cli, open_config_folder, popups::cli as popups_cli, weg::cli as weg_cli,
    },
};

/// WARNING: seelen-ui.exe CLI commands are deprecated.
///
/// Use `slu` instead.
#[derive(Debug, clap::Parser)]
#[command(version, name = "Seelen UI")]
struct MainCli {
    #[arg(long, default_value_t)]
    silent: bool,
    #[arg(long, default_value_t)]
    verbose: bool,
}

/// Called at startup by the main executable. Parses the flags and returns Ok(())
/// so normal app startup continues.
pub async fn handle_console_client() -> Result<()> {
    use clap::Parser;
    let cli = match MainCli::try_parse() {
        Ok(cli) => cli,
        Err(e) => e.exit(),
    };

    if cli.silent {
        crate::SILENT.store(true, Ordering::SeqCst);
    }
    if cli.verbose {
        crate::VERBOSE.store(true, Ordering::SeqCst);
        println!("Received args: {:#?}", std::env::args().collect::<Vec<_>>());
        println!("Parsed CLI: {cli:#?}");
    }

    Ok(())
}

pub async fn process_app_command(cmd: AppCommand) -> Result<()> {
    match cmd {
        AppCommand::Settings => {
            open_config_folder()?;
        }
        AppCommand::VirtualDesk(command) => {
            vd_cli::process(command)?;
        }
        AppCommand::Debugger(command) => {
            debugger::process(command)?;
        }
        AppCommand::Weg(command) => {
            weg_cli::process(command)?;
        }
        AppCommand::Widget(command) => {
            widget_cli::run(command)?;
        }
        AppCommand::Resource(command) => {
            resources_cli::process(command).await?;
        }
        AppCommand::Popup(command) => {
            popups_cli::process(command)?;
        }
        AppCommand::ToggleShortcutsPause => {
            shortcuts::toggle_pause()?;
        }
        _ => {
            return Err("Command does not support instance execution".into());
        }
    }
    Ok(())
}
