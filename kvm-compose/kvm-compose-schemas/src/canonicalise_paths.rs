use std::fs;
use crate::cli_models::{DeploymentSubCommand, Opts, SubCommand};
use crate::exec::{ExecCmdType, TestbedTools};

/// Canonicalise all the paths that are used in the CLI based on the working directory. This is
/// necessary as when the configuration is sent to the server, the server will not know the absolute
/// file paths for any inputs given by the CLI.
pub fn cli_canonicalise_all_paths(
    opts: &mut Opts,
) -> anyhow::Result<()> {

    // make sure the path to the kvm-compose yaml is absolute
    opts.input = fs::canonicalize(&opts.input)?
        .to_string_lossy()
        .into_owned();

    // depending on the command, there will be different canonicalisation requirements
    // we will also leave this without the _ => {} catch all in case in the future we add a new
    // command, we will not forget to make any canonicalisation adjustments for it
    match &mut opts.sub_command {
        SubCommand::GenerateArtefacts => {}
        SubCommand::ClearArtefacts => {}
        SubCommand::CloudImages => {}
        SubCommand::SetupConfig => {}
        SubCommand::Deployment(deployment) => {
            match deployment.sub_command {
                DeploymentSubCommand::Create(_) => {}
                DeploymentSubCommand::Destroy(_) => {}
                DeploymentSubCommand::List => {}
                DeploymentSubCommand::Info(_) => {}
                DeploymentSubCommand::ResetState(_) => {}
            }
        }
        SubCommand::Up(_) => {}
        SubCommand::Down => {}
        SubCommand::Snapshot(_) => {}
        SubCommand::Tools(_) => {}
        SubCommand::TestbedSnapshot(_) => {}
        SubCommand::Exec(exec) => {
            match &mut exec.command_type {
                ExecCmdType::ShellCommand(_) => {}
                ExecCmdType::Push(transfer) => {
                    transfer.source_path = fs::canonicalize(&transfer.source_path)?;
                }
                ExecCmdType::Pull(transfer) => {
                    transfer.target_path = fs::canonicalize(&transfer.target_path)?;
                }
                ExecCmdType::Tool(tool) => {
                    match &mut tool.tool {
                        TestbedTools::ADB(_) => {}
                        TestbedTools::FridaSetup => {}
                        TestbedTools::TestPermissions(_) => {}
                        TestbedTools::TestPrivacy(_) => {}
                        TestbedTools::TLSIntercept(_) => {}
                    }
                }
            }
        }
    }


    Ok(())
}
