use tokio::fs::create_dir;
use std::io::ErrorKind;
use std::path::Path;
use chrono::{DateTime, Duration, Utc};
use tokio_cron_scheduler::{Job, JobScheduler};
use tracing::level_filters;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;
use regex::Regex;
use tracing_appender::non_blocking::WorkerGuard;
use kvm_compose_schemas::TESTBED_SETTINGS_FOLDER;

pub async fn configure_logging(

) -> WorkerGuard {
    // setup logging - leave thread info until public release
    let stdout_log = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_thread_ids(true)
        .with_thread_names(true);
    // .pretty();

    let log_folder = create_dir(format!("{TESTBED_SETTINGS_FOLDER}/log/")).await;
    match log_folder {
        Ok(_) => {}
        Err(err) => match err.kind() {
            ErrorKind::PermissionDenied => {
                panic!("permission denied creating log folder, is the server running as root?");
            }
            ErrorKind::AlreadyExists => {}
            _ => {}
        },
    }

    let file_appender = tracing_appender::rolling::daily(format!("{TESTBED_SETTINGS_FOLDER}/log/"), "server.log");
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    #[cfg(debug_assertions)]
    let log_level = level_filters::LevelFilter::DEBUG;
    #[cfg(not(debug_assertions))]
    let log_level = level_filters::LevelFilter::INFO;

    // main log file, debug for now
    let debug_log = tracing_subscriber::fmt::layer()
        .with_ansi(false)
        .with_writer(non_blocking)
        .with_filter(log_level);

    // set up logging for the layers and push logs into a file
    tracing_subscriber::registry()
        .with(
            stdout_log
                // Add an `INFO` filter to the stdout logging layer
                .with_filter(log_level)
                // Combine the filtered `stdout_log` layer with the
                // `debug_log` layer, producing a new `Layered` layer.
                .and_then(debug_log),
        )
        .init();
    guard
}

/// Orchestration actions by the server will place logs in `/var/lib/testbedos/log/orchestration/`.
/// This cron job will check for old logs and clean them up to prevent saving then indefinitely.
pub async fn setup_orchestration_log_cleanup(

) -> anyhow::Result<()> {
    // set up cron job to monitor clients
    let sched = JobScheduler::new().await?;
    sched.add(
        Job::new_async("1/10 * * * * *", |_uuid, _l| Box::pin( async move {

            tracing::debug!("running orchestration log cleanup");
            let log_folder = format!("{TESTBED_SETTINGS_FOLDER}/log/orchestration/");
            let paths = match std::fs::read_dir(Path::new(&log_folder)) {
                Ok(ok) => ok,
                Err(err) => {
                    tracing::error!("cleanup cronjob error: {err:#}");
                    return;
                }
            };
            let file_names = paths.filter_map(|entry| {
                // check if file name read was successful, then get the filename and convert to String
                entry.ok().and_then(|e| {
                    e.path().file_name().and_then(|n| n.to_str().map(|s| String::from(s)))
                })
            }).collect::<Vec<String>>();
            // with the file names, extract datetime and check if old enough to delete
            let re = Regex::new(r"(\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}\.\d{9}\+\d{2}:\d{2})").unwrap();
            for log_file in file_names {
                // if it does not have a datetime in string, ignore it
                if let Some(datetime) = re.captures(&log_file) {
                    let found_log_file_timestamp = datetime.get(1).unwrap().as_str();
                    tracing::debug!("found log file: {}", found_log_file_timestamp);
                    // check if it is old enough to delete
                    let datetime = DateTime::parse_from_str(found_log_file_timestamp, "%Y-%m-%dT%H:%M:%S%.f%:z")
                        .expect("Failed to parse datetime")
                        .with_timezone(&Utc);
                    let one_week_ago = Utc::now() - Duration::weeks(1);
                    if datetime < one_week_ago {
                        tracing::info!("log file {} over a week old, deleting..", &log_file);
                        match std::fs::remove_file(format!("{TESTBED_SETTINGS_FOLDER}/log/orchestration/{log_file}")) {
                            Ok(_) => {}
                            Err(err) => {
                                tracing::error!("could not delete {log_file} with err: {err:#}");
                            }
                        }
                    } else {
                        tracing::debug!("log file {} under a week old, will not delete", &log_file);
                    }

                } else {
                    tracing::warn!("orchestration log file did not match regex: {}", &log_file);
                }
            }

        }))?
    ).await?;
    sched.start().await?;
    Ok(())
}
