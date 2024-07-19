use std::path::PathBuf;

pub mod artefact_generation;
pub mod cloud_init;
pub mod clones;
pub mod xml;
pub mod serialisation;
pub mod android;

pub fn check_file_exists(
    file_path: &String,
) -> bool {
    let file_path = PathBuf::from(file_path);
    file_path.exists()
}
