use anyhow::bail;
use async_trait::async_trait;
use kvm_compose_schemas::settings::{SshConfig, TestbedClusterConfig};

/// The `TestbedConfigProvider` is a trait to describe the database that backs the config
/// of the testbed and the actions possible. This covers the various configs that exist for the
/// testbed server.
#[async_trait]
pub trait TestbedConfigProvider {
    async fn get_cluster_config(&self) -> anyhow::Result<TestbedClusterConfig>;
    async fn set_cluster_config(&self, config: TestbedClusterConfig) -> anyhow::Result<()>;

    async fn get_host_config(&self) -> anyhow::Result<SshConfig>;
    async fn set_host_config(&self, config: SshConfig) -> anyhow::Result<()>;
}

/// This enum wraps the `TestbedClusterConfigProvider` implementations to be called in the server
/// initialisation.
#[derive(Clone)]
pub enum TestbedConfigDatabaseProvider {
    FileDB,
    // SQLite.
}

impl TestbedConfigDatabaseProvider {
    /// Return the config database boxed with sync and send so that it is thread safe
    pub fn get_provider(self) -> Box<dyn TestbedConfigProvider + Sync + Send> {
        match &self {
            TestbedConfigDatabaseProvider::FileDB { .. } => {
                tracing::info!("using the file based providers as a database for testbed config");
                Box::new(self)
            }
        }
    }
}

#[async_trait]
impl TestbedConfigProvider for TestbedConfigDatabaseProvider {
    async fn get_cluster_config(&self) -> anyhow::Result<TestbedClusterConfig> {
        match &self {
            TestbedConfigDatabaseProvider::FileDB => {
                let config = TestbedClusterConfig::read().await;
                match config {
                    Ok(ok) => Ok(ok),
                    Err(err) => {
                        bail!("could not get cluster config, err: {err:#}");
                    }
                }
            }
        }
    }

    async fn set_cluster_config(&self, config: TestbedClusterConfig) -> anyhow::Result<()> {
        match &self {
            TestbedConfigDatabaseProvider::FileDB => {
                let res = config.write().await;
                match res {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        bail!("could not set cluster config, err: {err:#}");
                    }
                }
            }
        }
    }

    async fn get_host_config(&self) -> anyhow::Result<SshConfig> {
        match &self {
            TestbedConfigDatabaseProvider::FileDB => {
                let config = SshConfig::read().await;
                match config {
                    Ok(ok) => Ok(ok),
                    Err(err) => {
                        bail!("could not get host config, err: {err:#}");
                    }
                }
            }
        }
    }

    async fn set_host_config(&self, config: SshConfig) -> anyhow::Result<()> {
        match &self {
            TestbedConfigDatabaseProvider::FileDB => {
                let res = config.write().await;
                match res {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        bail!("could not set host config, err: {err:#}");
                    }
                }
            }
        }
    }
}