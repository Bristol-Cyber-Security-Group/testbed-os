use crate::config::provider::{TestbedConfigDatabaseProvider, TestbedConfigProvider};

pub fn get_cluster_config_db() -> Box<dyn TestbedConfigProvider + Sync + Send> {
    // TODO - if we implement SQLite provider then we need to update this with a match
    TestbedConfigDatabaseProvider::FileDB.get_provider()
}