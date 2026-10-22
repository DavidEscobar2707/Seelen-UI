mod art;
mod resources;

use clap::Parser;
use slu_ipc::{
    AppIpc,
    commands::{AppCli, AppCommand, CommandExecutionMode, SluCliCommand},
    messages::AppMessage,
};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

fn main() {
    let cli = AppCli::parse();

    // the watchdog just blocks on a process handle: no async runtime (keeps it a tiny process)
    if let AppCommand::TaskbarWatchdog(cmd) = &cli.command {
        slu_ipc::taskbar_watchdog::run(cmd.pid, cmd.state, &cmd.file);
        return;
    }

    let rt = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .expect("failed to build runtime");
    rt.block_on(async_main(cli));
}

async fn async_main(cli: AppCli) {
    if cli.verbose {
        println!("Received args: {:#?}", std::env::args().collect::<Vec<_>>());
        println!("Parsed CLI: {cli:#?}");
    }

    if let Err(err) = run(cli).await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run(cli: AppCli) -> Result<()> {
    let mode = cli.command.execution_mode();
    match mode {
        CommandExecutionMode::Direct => process_direct(cli).await,
        CommandExecutionMode::MainInstance => send_to_main_instance(cli).await,
    }
}

async fn process_direct(cli: AppCli) -> Result<()> {
    match cli.command {
        AppCommand::Art(cmd) => art::process(cmd),
        AppCommand::Resource(cmd) => resources::process(cmd).await?,
        _ => return Err("Command does not support direct execution".into()),
    }
    Ok(())
}

async fn send_to_main_instance(cli: AppCli) -> Result<()> {
    let working_dir = std::env::current_dir()?;
    let args: Vec<String> = std::env::args()
        .map(|arg| {
            if arg.starts_with("./")
                || arg.starts_with(".\\")
                || arg.starts_with("../")
                || arg.starts_with("..\\")
            {
                working_dir.join(&arg).to_string_lossy().to_string()
            } else {
                arg
            }
        })
        .collect();

    if cli.verbose {
        println!("Sending {args:#?}");
    }

    AppIpc::send(AppMessage::Cli(args)).await?;
    Ok(())
}
