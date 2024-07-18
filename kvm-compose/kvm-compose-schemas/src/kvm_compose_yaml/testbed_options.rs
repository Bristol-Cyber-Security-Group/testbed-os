use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize, Serialize, Debug, Clone)]
pub struct TestbedOptions {
    pub load_balancing: LoadBalancing,
}

#[derive(Debug, Default, Clone, Deserialize, Serialize)]
pub enum LoadBalancing {
    /// This algorithm just allocates a guest to each testbed host without considering any host
    /// resources available, in a round-robin order.
    NaiveRoundRobin,
    // /// This algorithm will allocate guests on the main testbed host first until no more can be
    // /// allocated, then fill the next testbed host in the list until no more, then the next etc.
    // MainFirst,
    /// This algorithm will allocate guests evenly to maximise free resources on all testbed hosts.
    #[default]
    Balanced,
    // /// This setting allows the user to specify which host the guest will be assigned to. Up to the
    // /// user to manage memory usage.
    // Manual,
}

