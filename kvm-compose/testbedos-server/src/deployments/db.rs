use kvm_compose_schemas::TESTBED_SETTINGS_FOLDER;
use crate::deployments::providers::{DeploymentDatabaseProvider, DeploymentProvider, FileBasedProvider};

pub fn get_deployment_db() -> Box<dyn DeploymentProvider + Sync + Send> {
    let db = DeploymentDatabaseProvider::FileDB(FileBasedProvider {
        data_location: format!("{TESTBED_SETTINGS_FOLDER}/deployments/"),
    })
        .get_provider();
    db
}
