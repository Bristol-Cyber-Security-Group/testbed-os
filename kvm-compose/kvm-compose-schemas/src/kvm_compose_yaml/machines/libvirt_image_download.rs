use serde::{Deserialize, Serialize};
use std::path::Path;
use std::path::PathBuf;
use futures::StreamExt;
use strum::IntoEnumIterator;
use strum_macros::EnumIter;

#[derive(Debug, Deserialize, Serialize, EnumIter, Clone)]
#[serde(rename_all = "lowercase")]
#[allow(non_camel_case_types)]
// Currently supported image types
pub enum OnlineCloudImage {
    Ubuntu_18_04,
    Ubuntu_20_04,
    Ubuntu_22_04,
    Cirros_0_6_2,
}

impl OnlineCloudImage {
    // For any item within the enum provide a url to fetch the disk image
    fn get_url(&self) -> &str {
        match &self {
            OnlineCloudImage::Ubuntu_18_04 => {
                "https://cloud-images.ubuntu.com/bionic/20230607/bionic-server-cloudimg-amd64.img"
            }
            OnlineCloudImage::Ubuntu_20_04 => {
                "https://cloud-images.ubuntu.com/focal/20240430/focal-server-cloudimg-amd64.img"
            }
            OnlineCloudImage::Ubuntu_22_04 => {
                "https://cloud-images.ubuntu.com/jammy/20240426/jammy-server-cloudimg-amd64.img"
            }
            OnlineCloudImage::Cirros_0_6_2 => {
                "https://download.cirros-cloud.net/0.6.2/cirros-0.6.2-x86_64-disk.img"
            }
        }
    }

    pub fn get_os_variant(&self) -> String {
        match &self {
            OnlineCloudImage::Ubuntu_18_04 => "ubuntubionic".to_string(),
            OnlineCloudImage::Ubuntu_20_04 => "ubuntufocal".to_string(),
            OnlineCloudImage::Ubuntu_22_04 => "ubuntujammy".to_string(),
            OnlineCloudImage::Cirros_0_6_2 => unimplemented!(),
        }
    }

    pub async fn resolve_path(&self, storage_location: PathBuf) -> anyhow::Result<PathBuf> {
        let mut name = storage_location;
        name.push(format!("{}.img", serde_plain::to_string(self)?));
        // TODO - check file integrity in case the download didnt work
        if !name.is_file() {
            download_file(self.get_url(), &name).await?;
        }
        Ok(name)
    }

    pub fn print_image_list() {
        tracing::info!("Available cloud images:");
        for i in Self::iter() {
            tracing::info!("{}", serde_plain::to_string(&i).unwrap());
        }

    }

    /// Used to get the cloud images as a string rather than logging direct
    pub fn pretty_to_string() -> anyhow::Result<Vec<String>> {
        let mut images = Vec::new();
        for i in Self::iter() {
            images.push(serde_plain::to_string(&i)?);
        }
        Ok(images)
    }

    // pub fn get_cloud_init_type(&self) -> CloudInitType {
    //     match &self {
    //         OnlineCloudImage::Cirros_0_5_1 => CloudInitType::CirrosInit,
    //         _ => CloudInitType::CloudInit,
    //     }
    // }
}

pub async fn download_file(url: &str, destination: &Path) -> anyhow::Result<()> {
    tracing::info!("Downloading cloud image {}", url);

    // make sure images folder exists
    let images_folder = Path::new("/var/lib/testbedos/images/");
    if !images_folder.exists() {
        tokio::fs::create_dir(images_folder).await?;
    }

    let mut tmp_file = tokio::fs::File::create(destination).await?;
    let mut byte_stream = reqwest::get(url).await?.bytes_stream();

    while let Some(item) = byte_stream.next().await {
        tokio::io::copy(&mut item?.as_ref(), &mut tmp_file).await?;
    }

    Ok(())
}
