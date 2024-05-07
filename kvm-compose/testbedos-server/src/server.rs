use std::process::exit;
use axum::{Router, routing::{get, post}, ServiceExt};
use std::net::SocketAddr;
use tokio::process::{Command};
use std::sync::Arc;
use axum::extract::Request;
use http::{HeaderValue, Method};
use sysinfo::{System};
use tera::Tera;
use tokio::net::TcpListener;
use tokio::sync::RwLock;
use tower_http::cors::{CorsLayer};
use tower_http::normalize_path::NormalizePathLayer;
use tower_layer::Layer;
use testbedos_lib;
use testbedos_lib::{AppState, ClientAppState, ip_string_to_slice, logging, ServiceClients};
use testbedos_lib::cluster::{create_config_wizard, parse_cli_args, ServerModeCmd};
use testbedos_lib::cluster::cluster::configure_testbed_host;
use testbedos_lib::deployments::db::get_deployment_db;
use testbedos_lib::config::handlers::*;
use testbedos_lib::deployments::handlers::*;
use testbedos_lib::deployments::providers::DeploymentProvider;
use testbedos_lib::cluster::handlers::{check_membership, join_cluster};
use testbedos_lib::cluster::ovn::{set_up_cluster_client_check_cron_jobs, set_up_cluster_master_check_cron_jobs};
use testbedos_lib::config::db::get_cluster_config_db;
use testbedos_lib::config::provider::TestbedConfigProvider;
use testbedos_lib::gui::add_gui_handlers;
use testbedos_lib::logging::setup_orchestration_log_cleanup;
use testbedos_lib::orchestration::add_orchestration_handlers;
use testbedos_lib::resource_monitoring::handlers::*;

// we use a couple of threads, arbitrarily set to 4 as modern cpus are usually now at least 4 cores.
// the testbed is going to be handling quite a few requests when dealing with resource monitoring,
// while the server can handle this with one thread in the async runtime, we will give ourselves
// some space as watching the logs has shown the work stealing threads to manage to steal work,
// meaning the threads were slow enough to have work "stolen" from them in the runtime
#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
async fn main() {

    let _logging_guard = logging::configure_logging().await;

    // take CLI arguments to start the server in a mode
    let mode = match parse_cli_args().await {
        Ok(ok) => ok,
        Err(err) => {
            tracing::error!("could not start server due to start mode configuration, error: {err:#}");
            exit(1);
        },
    };

    // store address to server
    // listen on all addresses as CLI and client testbed hosts will use different IPs
    let ip = "0.0.0.0".to_string();
    let server_url = String::from(format!("http://{ip}:3355"));
    let addr = SocketAddr::from((
        ip_string_to_slice(&ip).expect(&format!("getting ip u8 slice from {ip}")),
        3355,
    ));

    // run server in the mode supplied by CLI arguments or via the default mode in the settings file
    // at /var/lib/testbedos/config/mode.json
    match mode {
        ServerModeCmd::Master => {
            // setup provider for state, this could be interchangeable to different databases
            // the provider will implement DeploymentProvider trait to make database calls available
            // inside the handlers - this should become a match statement when multiple providers exist
            let deployment_db = get_deployment_db();
            let config_db = get_cluster_config_db();
            // the database is wrapped in a read/write lock to prevent race conditions - the handlers will
            //  request the appropriate lock in their context
            // the database is also wrapped in an atomically referenced counter to ensure there is only one
            //  between all handler contexts
            let deployment_config_db: Arc<RwLock<Box<(dyn DeploymentProvider + Sync + Send)>>> =
                Arc::new(RwLock::new(deployment_db));
            let config_db: Arc<RwLock<Box<(dyn TestbedConfigProvider + Sync + Send)>>> =
                Arc::new(RwLock::new(config_db));

            // given the mode, make sure settings are correct
            try_configure_host(&mode, &config_db).await;

            // the app state contains any shared context for the handlers
            let app_state = Arc::new(AppState {
                deployment_config_db,
                config_db,
                server_url,
                system_monitor: Arc::new(RwLock::new(System::new_all())),
                template_env: get_tera_env(),
                service_clients: Arc::new(ServiceClients::new().await),
            });

            // TODO - use router combination syntax?
            // build routers for server endpoints
            // add trim slash middleware
            let app = NormalizePathLayer::trim_trailing_slash().layer(master_app(app_state));

            // set up cron job to monitor clients
            match set_up_cluster_client_check_cron_jobs().await {
                Ok(_) => {}
                Err(err) => {
                    tracing::error!("could not set up cluster cron job with err: {err:#}");
                    exit(1);
                }
            }
            // set up cron job to clear orchestration logs
            match setup_orchestration_log_cleanup().await {
                Ok(_) => {}
                Err(err) => {
                    tracing::error!("could not set up orchestration log cron job with err: {err:#}");
                    exit(1);
                }
            }

            // TODO - run the server on a socket so that the user must have sudo rights since the supporting
            //  software such as libvirt also requires sudo - or create a testbedOS group to cover all
            // Run our app with hyper
            tracing::info!("listening on {} in master mode", addr);
            let listener = TcpListener::bind(&addr).await.unwrap();
            axum::serve::serve(listener, ServiceExt::<Request>::into_make_service_with_connect_info::<SocketAddr>(app))
                .await
                .unwrap();
        }
        ServerModeCmd::Client(ref client_mode) => {
            // start server in client mode
            let config_db = get_cluster_config_db();
            let config_db: Arc<RwLock<Box<(dyn TestbedConfigProvider + Sync + Send)>>> =
                Arc::new(RwLock::new(config_db));
            let app_state = Arc::new(ClientAppState {
                config_db,
                master_server_url: client_mode.master_ip.clone(),
                system_monitor: Arc::new(RwLock::new(System::new_all())),
                service_clients: Arc::new(ServiceClients::new().await)
            });
            // given the mode, make sure settings are correct
            try_configure_host(&mode, &app_state.config_db).await;
            // set up cron job to check master is online
            match set_up_cluster_master_check_cron_jobs(
                client_mode.master_ip.clone(),
            ).await {
                Ok(_) => {}
                Err(err) => {
                    tracing::error!("could not set up cluster cron jobs with err: {err:#}");
                    exit(1);
                }
            }
            // build routers for server endpoints
            let app = client_app(app_state);
            tracing::info!("listening on {} in client mode with remote server {}", addr, &client_mode.master_ip);
            let listener = TcpListener::bind(&addr).await.unwrap();
            axum::serve::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>())
                .await
                .unwrap();
        }
        ServerModeCmd::CreateConfig => {
            // take user through creating a config
            create_config_wizard();
            tracing::info!("restarting testbed server daemon to reflect changes");
            let output = Command::new("sudo")
                .arg("systemctl")
                .arg("restart")
                .arg("testbedos-server.service")
                .output()
                .await;
            match output {
                Ok(_) => tracing::info!("testbed server restarted"),
                Err(err) => {
                    tracing::error!("error restarting testbed server {err:#}");
                    exit(1);
                }
            }
            tracing::info!("wizard finished, exiting");
            exit(0)
        }
    }
}

/// Produce the app in a separate function to allow for testing without creating an http server.
/// This also represents the master mode set of urls.
pub fn master_app(app_state: Arc<AppState>) -> Router {
    Router::new()
        .route(
            "/api/config/cluster",
            get(get_testbed_cluster_config).post(set_testbed_cluster_config),
        )
        .route(
            "/api/config/host", get(get_testbed_host_config).post(set_testbed_host_config),
        )
        .route("/api/config/status", get(host_status))
        .route("/api/config/default", get(get_default_host_json))
        .route("/api/config/usergroupqemu", get(get_qemu_conf_user_group))
        .route(
            "/api/cluster",
            post(join_cluster)
        )
        .route("/api/cluster/:name", get(check_membership))
        .route("/api/validate/yaml", post(validate_yaml_endpoint))
        .route("/api/validate/projectname", post(validate_project_name_handler))
        .route(
            "/api/deployments",
            get(list_deployments).post(create_deployment),
        )
        .route("/api/deployments/:name/yaml", get(get_deployment_yaml))
        .route("/api/active-deployments", get(list_active_deployments))
        .route(
            "/api/deployments/:name",
            get(get_deployment)
                .delete(delete_deployment)
                .put(update_deployment),
        )
        // .route("/api/deployments/:name/action", post(action_deployment))
        .route("/api/deployments/:name/state", get(get_state))
        .route("/api/metrics/prometheus/hosts", get(prometheus_scrape_endpoint_for_hosts))
        .route("/api/metrics/prometheus/libvirt", get(prometheus_scrape_endpoint_for_libvirt))
        .route("/api/metrics/prometheus/android", get(prometheus_scrape_endpoint_for_android))
        .route("/api/metrics/prometheus/docker", get(prometheus_scrape_endpoint_for_docker))
        .route("/api/metrics/host", get(get_master_testbed_host_resource))
        .route("/api/metrics/state", get(get_metrics_state))
        .route("/api/metrics/guest/:project/:name", get(get_master_testbed_guest_resource))
        .route("/api/metrics/dashboard/:project", get(resource_monitoring_dashboard))
        .nest("/api/orchestration", add_orchestration_handlers())
        .nest("/", add_gui_handlers())
        .layer(CorsLayer::new()
            .allow_origin("http://localhost:8080".parse::<HeaderValue>().unwrap())
            .allow_methods([Method::GET, Method::POST]))
        .with_state(app_state)
}

pub fn client_app(app_state: Arc<ClientAppState>) -> Router {
    // TODO - add client mode routes
    Router::new()
        .route("/api/config/status", get(host_status))
        .route("/api/metrics/host", get(get_client_testbed_host_resource))
        .route("/api/metrics/guest/:project/:name", get(get_client_testbed_guest_resource))
        .with_state(app_state)
}

async fn try_configure_host(
    mode: &ServerModeCmd,
    db_config: &Arc<RwLock<Box<dyn TestbedConfigProvider + Sync + Send>>>,
) {
    match configure_testbed_host(&mode, &db_config).await {
        Ok(_) => { }
        Err(err) => {
            tracing::error!("could not start server due host configuration issue, error: {err:#}");
            exit(1);
        }
    };
}

/// Use a fixed location for templates if running in release mode, since there is no guarantee the
/// working directory of the testbed is in the correct location and the templates are not inserted
/// into the binary. For debug mode, we use the git repo the testbed is running from.
fn get_tera_env() -> Arc<RwLock<Tera>> {
    #[cfg(debug_assertions)]
    let env = Arc::new(RwLock::new(Tera::new("assets/templates/**/*.html")
        .expect("could not load Tera templates")));

    #[cfg(not(debug_assertions))]
    let env = Arc::new(RwLock::new(Tera::new("/var/lib/testbedos/assets/templates/**/*.html")
        .expect("could not load Tera templates")));

    env
}
