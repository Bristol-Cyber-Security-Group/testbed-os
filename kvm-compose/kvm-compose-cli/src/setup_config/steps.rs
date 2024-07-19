use ratatui::{Frame, Terminal};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::backend::CrosstermBackend;
use std::io::Stdout;
use ratatui::prelude::*;
use crate::setup_config;
use crate::setup_config::{ControlFlow, must_be_sudo, SetupConfigState, welcome};

/// This enum represents all the steps in the setup config wizard
pub enum SetupSteps {
    /// First page to introduce the user
    Welcome,
    /// Set up the testbed host.json file
    HostJson,
    /// Ask the user if they would like to change the qemu.conf file to allow libvirt images outside
    /// the libvirt images folder (necessary for libvirt guests in testbed)
    QemuConf,
    /// Check if the system DNS is appropriate, if not give a warning and show where to go
    SystemDNS,
    /// Ask the user if they want the resource monitoring docker compose stack to be run in the
    /// background. Otherwise, give the command on how to do this.
    ToggleResourceMonitoring,
    /// When we get to this step, we can execute the configuration
    Execute,
    /// Error page instructing user to run command with sudo
    NotSudo,

    Exit,
}

impl SetupSteps {
    pub(crate) fn get_step_fn(step: &SetupSteps) -> impl FnOnce(&mut Frame) {
        match step {
            SetupSteps::Welcome => welcome,
            SetupSteps::HostJson => host_json,
            SetupSteps::QemuConf => qemu_conf,
            SetupSteps::SystemDNS => system_dns,
            SetupSteps::ToggleResourceMonitoring => toggle_resource_monitoring,
            SetupSteps::Execute => execute,
            SetupSteps::NotSudo => must_be_sudo,
            // Assumption that the caller will handle the Exit step explicitly and not call this
            // function on Exit variant
            SetupSteps::Exit => unreachable!(),
        }
    }

    pub(crate) fn get_next_step(current_step: &SetupSteps) -> SetupSteps {
        match current_step {
            SetupSteps::Welcome => SetupSteps::HostJson,
            SetupSteps::HostJson => SetupSteps::QemuConf,
            SetupSteps::QemuConf => SetupSteps::SystemDNS,
            SetupSteps::SystemDNS => SetupSteps::ToggleResourceMonitoring,
            SetupSteps::ToggleResourceMonitoring => SetupSteps::Execute,
            SetupSteps::Execute => SetupSteps::Exit,
            SetupSteps::NotSudo => SetupSteps::Exit,
            // Assumption that the caller will handle the Exit step explicitly and not call this
            // function on Exit variant
            _ => unreachable!(),
        }
    }
}

#[allow(dead_code)]
pub fn loop_step(
    current_step: &mut SetupSteps,
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    quit_or_continue: &mut ControlFlow,
    _setup_config_state: &mut SetupConfigState,
) -> anyhow::Result<()> {
    loop {
        let next_frame = SetupSteps::get_step_fn(current_step);
        terminal.draw(next_frame)?;
        *quit_or_continue = setup_config::handle_step_events()?;

        // TODO - break this out into functions
        match current_step {
            SetupSteps::HostJson => {

            }
            _ => {}
        }

        match quit_or_continue {
            ControlFlow::Quit => {
                *current_step = SetupSteps::Exit;
                break;
            }
            ControlFlow::Continue => {
                *current_step = SetupSteps::get_next_step(current_step);
                break;
            }
            ControlFlow::WaitForUser => {}
        }
    }
    Ok(())
}

#[allow(dead_code)]
fn host_json(frame: &mut Frame) {
    let area = frame.size();
    // layout config

    // outer - Left is distance from title, Right unclear (might be area for blocks below, everything but title)
    let main_layout = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]);

    // get the area for title and the main area
    let [title_area, main_area] = main_layout.areas(area);

    // split the main area into two more, we want to show some info text, then place the text input in the other area
    let info_layout = Layout::vertical([Constraint::Max(10), Constraint::Min(4)]);
    let [info_area, action_area] = info_layout.areas(main_area);

    // place a title
    frame.render_widget(
        Paragraph::new("Testbed Setup - host.json")
            .alignment(Alignment::Center),
        title_area,
    );

    // place some text in the info area
    let block = Block::new()
        .borders(Borders::ALL);
    frame.render_widget(
        Paragraph::new("You need to do ...")
            .alignment(Alignment::Left)
            .block(block),
        info_area,
    );

    // place something in the action area
    let block = Block::new()
        .borders(Borders::ALL);
    frame.render_widget(
        Paragraph::new("Some text input ...")
            .alignment(Alignment::Left)
            .block(block),
        action_area,
    );

}

#[allow(dead_code)]
fn qemu_conf(frame: &mut Frame) {
    frame.render_widget(
        Paragraph::new("TODO")
            .block(Block::bordered().title("Testbed Setup - qemu conf")),
        frame.size(),
    );
}

#[allow(dead_code)]
fn system_dns(frame: &mut Frame) {
    frame.render_widget(
        Paragraph::new("TODO")
            .block(Block::bordered().title("Testbed Setup - system dns")),
        frame.size(),
    );
}

#[allow(dead_code)]
fn toggle_resource_monitoring(frame: &mut Frame) {
    frame.render_widget(
        Paragraph::new("TODO")
            .block(Block::bordered().title("Testbed Setup - toggle resource monitoring")),
        frame.size(),
    );
}

#[allow(dead_code)]
fn execute(frame: &mut Frame) {
    frame.render_widget(
        Paragraph::new("TODO")
            .block(Block::bordered().title("Testbed Setup - execute")),
        frame.size(),
    );
}
