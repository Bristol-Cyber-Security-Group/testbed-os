mod common;

use anyhow::bail;
use tempfile::tempdir;
use crate::common::setup_file_based;


#[tokio::test]
async fn test_create_deployment_guard() -> anyhow::Result<()> {

    let deployment_db_dir = tempdir()?;
    let config_db = setup_file_based(deployment_db_dir.path().to_string_lossy()).await;

    // we will create a guard inside a context so that when we exit the guard is dropped and we can
    // check the data structure to see if there is still a lock
    
    // make sure we can't lock twice
    {
        let guard_option = config_db.try_lock_deployment("test1".to_string());
        assert!(guard_option.is_some());
        let guard_option2 = config_db.try_lock_deployment("test1".to_string());
        assert!(guard_option2.is_none());
    }

    // make sure the lock reports that it is locked
    {
        // check before making it
        assert!(!config_db.is_deployment_locked("test2"));

        let guard_option = config_db.try_lock_deployment("test2".to_string());
        assert!(guard_option.is_some());

        assert!(config_db.is_deployment_locked("test2"));
    }
    // then it should not be locked anymore
    assert!(!config_db.is_deployment_locked("test2"));

    // the RAII should have removed both elements so the hashset should be empty
    assert_eq!(config_db.active_deployments.lock().unwrap().len(), 0);
    
    Ok(())
}
