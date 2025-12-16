use crate::deployments::models::*;
use anyhow::{bail, Context};
use tokio::fs::{File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use async_trait::async_trait;
use tokio::io::AsyncWriteExt;
use tokio::sync::{RwLock, RwLockReadGuard, RwLockWriteGuard};
use kvm_compose_lib::state::State;
use crate::deployments::{get_state_json, set_state_json};

/// The `DeploymentProvider` is a trait to describe the database that backs the server. This is used
/// together with the `DatabaseProvider` enum, which will wrap the types of database implementation.
/// This must be implemented for any database type such that the rest of the server code will work
/// with a plug and play for different databases. The chosen provider will be made available to all
/// the servers handler's context. Note that these providers will have to be thread safe, as to
/// allow atomic transactions to the database.
#[async_trait]
pub trait DeploymentProvider {
    async fn list_deployments(&self) -> anyhow::Result<DeploymentList>;
    async fn get_deployment(&self, name: String) -> anyhow::Result<Deployment>;
    async fn create_deployment(&self, deployment: NewDeployment) -> anyhow::Result<()>;
    async fn update_deployment(&self, name: String, deployment: Deployment)
        -> anyhow::Result<Deployment>;
    async fn delete_deployment(&self, name: String) -> anyhow::Result<()>;
    async fn get_state(&self, name: String) -> anyhow::Result<State>;
    async fn set_state(&self, name: String, state: State) -> anyhow::Result<()>;
}

/// This enum wraps the `DeploymentProvider` implementations to be called in the server
/// initialisation.
#[derive(Clone)]
pub enum DeploymentDatabaseProvider {
    FileDB(FileBasedProvider),
    SQLite(SQLiteProvider), // TODO
}

impl DeploymentDatabaseProvider {
    /// This function will return the `DeploymentProvider` as `Sync` and `Send` to the server
    /// initialisation.
    pub fn get_provider(self) -> Box<dyn DeploymentProvider + Sync + Send> {
        // return the database provider as the trait so that we can use the
        // trait methods in the generic handlers so we can hot swap database
        // providers
        match self {
            DeploymentDatabaseProvider::FileDB(db) => {
                tracing::info!("using the file based provider as a database for deployments");
                Box::new(db)
            }
            DeploymentDatabaseProvider::SQLite(_db) => {
                tracing::info!("using the sqlite based provider as a database for deployments");
                todo!()
            }
        }
    }
}

/// The file based provider works completely with a series of files and folders in the servers
/// configuration folder. On every API request to the database, the files and folders are read on
/// demand and relevant information is returned or edited. This is a simple implementation to get
/// going with the server without needing to install database infrastructure. Unless there is a
/// significantly large number of deployments or generally files to read, then the performance of
/// this provider will be good.
#[derive(Clone)]
pub struct FileBasedProvider {
    pub data_location: String,
    pub lock: Arc<RwLock<()>>,
}

/// Implement the file based provider. Since we are working with both files and directories to
/// represent state, and since we are not going to have significant traffic, we will place a lock
/// on the provider generally. So every function that is implemented for ``DeploymentProvider`` will
/// either place a red or write lock at the start of the function. This allows us to be generic to
/// either the whole folder or to a specific file. This should prevent race conditions on reading
/// after a write.
impl FileBasedProvider {
    pub fn new(data_location: String) -> Self {
        Self {
            data_location,
            lock: Arc::new(Default::default()),
        }
    }
    async fn write_lock(&self) -> RwLockWriteGuard<'_, ()> {
        self.lock.write().await
    }

    async fn read_lock(&self) -> RwLockReadGuard<'_, ()> {
        self.lock.read().await
    }

    async fn write(&self, file_name: String, data: String, create: bool) -> anyhow::Result<()> {
        let _guard = self.write_lock().await;
        let mut output = if create {
            File::create(file_name)
                .await
                .context("creating deployment file")?
        } else {
            OpenOptions::new()
                .write(true)
                .truncate(true)
                .open(file_name)
                .await
                .context("opening deployment file")?

        };

        output.write_all(data
            .as_bytes())
            .await
            .context("writing to deployment file")?;
        output.sync_all()
            .await
            .context("syncing deployment file")?;

        Ok(())
    }

    async fn read<F: AsRef<Path>>(&self, file_name: F) -> anyhow::Result<Deployment> {
        let _guard = self.read_lock().await;
        let text = tokio::fs::read_to_string(file_name)
            .await
            .context("reading deployment file")?;
        let config: Deployment = serde_json::from_str(&text)
            .context("deserializing deployment file")?;
        Ok(config)
    }

}

#[async_trait]
impl DeploymentProvider for FileBasedProvider {
    async fn list_deployments(&self) -> anyhow::Result<DeploymentList> {
        // look through /var/lib/testbedos/deployments/ folder for the existing
        // deployments

        let mut paths = tokio::fs::read_dir(&self.data_location).await?;

        let mut deployment_list = DeploymentList {
            deployments: Default::default(),
        };

        loop {
            if let Some(path) = paths.next_entry().await? {
                let file = path.path();
                // ignore the logs json
                if file.display().to_string().ends_with("-logs.json") {
                    continue;
                }

                let deployment = self.read(&file).await;

                match deployment {
                    Ok(dep) => {
                        // is a validated project
                        deployment_list
                            .deployments
                            .insert(dep.name.clone(), dep);
                    }
                    Err(_) => {
                        // there is a file but it is not a deployment file
                        let file_loc = &file.display();
                        tracing::info!("could not read file {file_loc} as a Deployment config");
                    }
                }

            } else {
                break;
            }
        }

        Ok(deployment_list)
    }

    async fn get_deployment(&self, name: String) -> anyhow::Result<Deployment> {
        let root_path = &self.data_location;
        if name.eq("") {
            bail!("deployment name was empty");
        }
        let path = PathBuf::from(format!("{root_path}{name}.json"));

        let deployment = self.read(&path)
            .await
            .context(format!("reading deployment file {path:?}"))?;
        Ok(deployment)

    }

    async fn create_deployment(&self, new_deployment: NewDeployment) -> anyhow::Result<()> {
        let root_path = &self.data_location;

        let deployment_name = &new_deployment.name;

        // prevent overwriting an existing deployment
        let existing_deployment_check = self.get_deployment(deployment_name.clone());
        if existing_deployment_check.await.is_ok() {
            bail!("there is already a deployment with this name");
        }

        let path = new_deployment.project_location;
        let json_name = format!("{root_path}{deployment_name}.json");

        // make sure there is a project there
        let project_folder = PathBuf::from(&path);
        if project_folder.is_dir() {
            // folder exists, does a yaml exist?
            let project_yaml = PathBuf::from(format!("{path}/kvm-compose.yaml"));
            if !project_yaml.is_file() {
                bail!("there is no kvm-compose.yaml file in this project");
            }
        } else {
            bail!("there is no project folder at location specified");
        }

        // TODO - validate the yaml?

        let deployment = Deployment {
            name: deployment_name.clone(),
            project_location: path.clone(),
            state: DeploymentState::Down,
            last_action_uuid: None,
        };

        self.write(json_name, deployment.to_string(), true)
            .await
            .context("creating deployment file")?;

        Ok(())
    }

    async fn update_deployment(
        &self,
        name: String,
        deployment: Deployment,
    ) -> anyhow::Result<Deployment> {
        let root_path = &self.data_location;

        let json_name = format!("{root_path}{name}.json");

        self.write(json_name, deployment.to_string(), false)
            .await
            .context("creating deployment file")?;

        Ok(deployment)
    }

    async fn delete_deployment(&self, name: String) -> anyhow::Result<()> {
        let root_path = &self.data_location;
        let json_name = format!("{root_path}{name}.json");
        let log_json_name = format!("{root_path}{name}-logs.json");
        let deployment = self.get_deployment(name).await?;

        match deployment.state {
            DeploymentState::Up => {
                bail!("cannot delete a deployment that is in UP state")
            }
            DeploymentState::Running => {
                bail!("cannot delete a deployment that is in RUNNING state")
            }
            _ => {
                let _guard = self.write_lock().await;

                tokio::fs::remove_file(json_name).await?;
                // this may not exist if deployment never orchestrated, ignore fail
                let _ = tokio::fs::remove_file(log_json_name).await;

                Ok(())
            }
        }
    }

    async fn get_state(&self, name: String) -> anyhow::Result<State> {
        let deployment = self.get_deployment(name)
            .await.context("get deployment")?;
        let state_json = get_state_json(deployment)
            .await
            .context("get state json");
        state_json
    }

    async fn set_state(&self, name: String, state: State) -> anyhow::Result<()> {
        let deployment = self.get_deployment(name)
            .await.context("get deployment")?;
        set_state_json(deployment, state).await?;
        Ok(())
    }
}

#[derive(Clone)]
pub struct SQLiteProvider;

#[allow(unused)]
#[async_trait]
impl DeploymentProvider for SQLiteProvider {
    async fn list_deployments(&self) -> anyhow::Result<DeploymentList> {
        todo!()
    }

    async fn get_deployment(&self, name: String) -> anyhow::Result<Deployment> {
        todo!()
    }

    async fn create_deployment(&self, deployment: NewDeployment) -> anyhow::Result<()> {
        todo!()
    }

    async fn update_deployment(
        &self,
        name: String,
        deployment: Deployment,
    ) -> anyhow::Result<Deployment> {
        todo!()
    }

    async fn delete_deployment(&self, name: String) -> anyhow::Result<()> {
        todo!()
    }

    async fn get_state(&self, name: String) -> anyhow::Result<State> {
        todo!()
    }

    async fn set_state(&self, name: String, state: State) -> anyhow::Result<()> {
        todo!()
    }
}
