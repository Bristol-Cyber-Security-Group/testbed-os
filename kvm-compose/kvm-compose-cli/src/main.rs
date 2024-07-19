mod setup_config;

use anyhow::{anyhow, bail, Context};
use clap::Parser;
use tracing_subscriber::prelude::*;
use tracing::level_filters::LevelFilter;
use kvm_compose_lib::server_web_client::client;
use kvm_compose_schemas::cli_models::{Opts, SubCommand};
use kvm_compose_schemas::kvm_compose_yaml::machines::libvirt_image_download::OnlineCloudImage;
use reqwest::Client;
use crate::setup_config::setup_config;


#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {
    std::process::exit(match run_app().await {
        Ok(_) => 0,
        Err(err) => {
            tracing::error!("{:#}", err);
            1
        }
    });
}

fn log_level(s: &str) -> anyhow::Result<LevelFilter> {
    match s.to_lowercase().as_str() {
        "error" => Ok(LevelFilter::ERROR),
        "warn" => Ok(LevelFilter::WARN),
        "info" => Ok(LevelFilter::INFO),
        "trace" => Ok(LevelFilter::TRACE),
        _ => Err(anyhow!("Unknown Log LevelFilter {}", s)),
    }
}

/// This is the entrypoint in the kvm compose library, called by the kvm-compose main.rs
/// The CLI arguments will be processed and the action will be performed. The main execution will
/// occur in ``parse_config`` which builds the logical testbed, if it is a non-config action.
pub async fn run_app() -> Result<(), anyhow::Error> {
    // Invoke cli option parsing
    let opts: Opts = Opts::parse();
    let mut e = None;
    // Determine and set log level
    let level = match &opts.verbosity {
        None => LevelFilter::INFO,
        Some(x) => match log_level(x) {
            Ok(l) => l,
            Err(err) => {
                e = Some(err);
                LevelFilter::INFO
            }
        },
    };
    e.map(|e| tracing::warn!("{}", e));

    let stdout_log = tracing_subscriber::fmt::layer();
    tracing_subscriber::registry()
        .with(stdout_log.with_filter(level))
        .init();

    match parse_command(opts).await {
        Ok(_) => {}
        Err(err) => {
            tracing::error!("ERROR: {}", err);
            err.chain().skip(1).for_each(|cause| tracing::error!("because: {}", cause));
        }
    }
    Ok(())
}

/// This is the entrypoint for all commands
pub async fn parse_command(opts: Opts) -> anyhow::Result<()> {
    // first check if it was a command that does not need the server
    match &opts.sub_command {
        SubCommand::CloudImages => {
            OnlineCloudImage::print_image_list();
            return Ok(());
        }
        SubCommand::SetupConfig => {
            // setup_config().await?;
            tracing::warn!("this is currently unimplemented");
            return Ok(());
        }
        _ => {}
    }

    tracing::trace!("server connection = {:?}", opts.server_connection);
    let client = Client::new();
    // check server is running
    let conn_test = client.get(format!("{}api/", opts.server_connection))
        .send()
        .await
        .context("Connecting to testbed server");
    if conn_test.is_err() {
        bail!("could not connect to testbed server at {}, is it running?", opts.server_connection);
    }

    let sub_command = match &opts.sub_command {
        SubCommand::GenerateArtefacts => client::orchestration_action(&client, opts).await,
        SubCommand::ClearArtefacts => client::orchestration_action(&client, opts).await,
        SubCommand::Deployment(dep_cmd) => client::deployment_action(&client, &opts, dep_cmd).await,
        SubCommand::Up(_) => client::orchestration_action(&client, opts).await,
        SubCommand::Down => client::orchestration_action(&client, opts).await,
        SubCommand::Snapshot(_) => client::orchestration_action(&client, opts).await,
        SubCommand::AnalysisTools(_) => unimplemented!(), // unimplemented while we are reworking tcpdump
        SubCommand::TestbedSnapshot(_) => client::orchestration_action(&client, opts).await,
        SubCommand::Exec(_) => client::orchestration_action(&client, opts).await,
        _ => bail!("command not matched, please raise an issue"),
    };
    sub_command
        .context("running CLI command")?;
    Ok(())
}
