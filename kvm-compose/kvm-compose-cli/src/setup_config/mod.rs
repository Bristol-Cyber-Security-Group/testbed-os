mod steps;

use std::io::{self, stdout};

use crossterm::{
    event::{self, Event, KeyCode},
    ExecutableCommand,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};
use kvm_compose_schemas::settings::SshConfig;
use crate::setup_config::steps::SetupSteps;

/// This struct represents the state of the configuration, that is updated while we take the user
/// through the wizard. Once we get to the `Execute` step, we can then read this and execute the
/// desired configuration on the host to set up the testbed.
#[derive(Default)]
#[allow(dead_code)]
struct SetupConfigState {
    // for HostJson step
    host_json: SshConfig,
    // for QemuConf step
    update_qemu_conf_user: Option<String>,
    update_qemu_conf_group: Option<String>,
    // for SystemDNS step
    update_system_dns: bool,
    // for ToggleResourceMonitoring step
    turn_on_resource_monitoring: bool,
}

/// This will walk the user through configuring the testbed installation
#[allow(dead_code)]
pub async fn setup_config(

) -> anyhow::Result<()> {

    // construct tui
    enable_raw_mode()?;
    stdout().execute(EnterAlternateScreen)?;
    let mut terminal = Terminal::new(CrosstermBackend::new(stdout()))?;

    // get welcome step if sudo, otherwise give error page
    let mut current_step = check_if_sudo();
    // let mut current_step = SetupSteps::Welcome;

    // track whether a step should quit or continue, default to wait
    let mut quit_or_continue = ControlFlow::WaitForUser;

    // track the setup config state through the wizard
    let mut setup_config_state = SetupConfigState::default();

    // keep looping until an exit state is set
    loop {
        // immediately quit if on break step
        match current_step {
            SetupSteps::Exit => break,
            _ => {}
        }

        // handle each step, once each step is done, set the next step and exit the inner loop to
        // return to this outer loop ...
        // ratatui works by looping indefinitely printing the current frame over and over and
        // looking for user input via the "handle_*_event" where we implement the behaviour
        match &current_step {
            SetupSteps::Welcome => steps::loop_step(&mut current_step, &mut terminal, &mut quit_or_continue, &mut setup_config_state)?,
            SetupSteps::HostJson => steps::loop_step(&mut current_step, &mut terminal, &mut quit_or_continue, &mut setup_config_state)?,
            SetupSteps::QemuConf => steps::loop_step(&mut current_step, &mut terminal, &mut quit_or_continue, &mut setup_config_state)?,
            SetupSteps::SystemDNS => steps::loop_step(&mut current_step, &mut terminal, &mut quit_or_continue, &mut setup_config_state)?,
            SetupSteps::ToggleResourceMonitoring => steps::loop_step(&mut current_step, &mut terminal, &mut quit_or_continue, &mut setup_config_state)?,
            SetupSteps::Execute => steps::loop_step(&mut current_step, &mut terminal, &mut quit_or_continue, &mut setup_config_state)?,
            SetupSteps::NotSudo => {
                loop {
                    terminal.draw(must_be_sudo)?;
                    let should_quit = handle_welcome_events()?;
                    if should_quit {
                        current_step = SetupSteps::Exit;
                        break;
                    }
                }
            }
            SetupSteps::Exit => break, // redundant
        }
    }

    // destroy tui
    disable_raw_mode()?;
    stdout().execute(LeaveAlternateScreen)?;

    Ok(())
}

fn handle_step_events() -> io::Result<ControlFlow> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(ControlFlow::Quit);
            }
            // catch anything but q
            if key.kind == event::KeyEventKind::Press && key.code != KeyCode::Char('q') {
                return Ok(ControlFlow::Continue);
            }
        }
    }
    Ok(ControlFlow::WaitForUser)
}

enum ControlFlow {
    Quit,
    Continue,
    WaitForUser,
}

#[allow(dead_code)]
fn handle_welcome_events() -> io::Result<bool> {
    if event::poll(std::time::Duration::from_millis(50))? {
        if let Event::Key(key) = event::read()? {
            if key.kind == event::KeyEventKind::Press && key.code == KeyCode::Char('q') {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

pub fn welcome(frame: &mut Frame) {
    frame.render_widget(
        Paragraph::new("Welcome to the Testbed Configuration Wizard.\n\
        We will walk through various steps to make sure the testbed is ready to go.\n\
        \n\
        You can press 'q' at any time to quit, you will lose any progress.\n\
        \n\
        Press any other key to continue.
        ")
            .block(Block::bordered().title("Testbed Setup")),
        frame.size(),
    );
}

pub fn must_be_sudo(frame: &mut Frame) {
    frame.render_widget(
        Paragraph::new(
            "You must run this command with sudo, as we will need to read and edit system configuration.\n\
            \n\
            Instead, please run 'sudo kvm-compose setup-config'.\n\
            \n\
            Press 'q' to quit."
        ).block(Block::bordered().title("Testbed Setup - ERROR")),
        frame.size(),
    );
}

#[allow(dead_code)]
fn check_if_sudo() -> SetupSteps {
    let uid = nix::unistd::Uid::current();
    if uid.is_root() {
        SetupSteps::Welcome
    } else {
        SetupSteps::NotSudo
    }
}