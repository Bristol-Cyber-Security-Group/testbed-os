use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize, Debug, Clone)]
pub struct TestbedOptions {
    pub load_balancing: LoadBalancing,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub enum LoadBalancing {
    #[default]
    NaiveRoundRobin,
}

