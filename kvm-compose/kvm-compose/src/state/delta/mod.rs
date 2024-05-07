use crate::state::State;

/// This struct and it's implementation will figure out the different between two `State` structs.
/// This is an important step in preventing state drift
pub struct StateDelta {

}

impl StateDelta {
    pub fn new(_old_state: &State, _new_state: &State) -> Self {
        todo!()
    }

    // TODO - start simple, work out if guests have been added/removed and if they have moved
    //  testbed hosts
    // TODO - we can look at the configuration of each guest to see if changed, including
    //  any scripts and mounts etc
    // TODO - network, bridges may or may not have changed etc which could impact guests

}
