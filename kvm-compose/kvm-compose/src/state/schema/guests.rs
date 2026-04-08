use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StateGuestType {

}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct StateLibvirtMachine {

}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "snake_case")]
pub enum StateLibvirtGuestOptions {

}

