use std::collections::HashMap;
use pueue_lib::task::Task;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct PueueStatus {
    pub groups: HashMap<String, PueueGroup>,
    pub tasks: HashMap<String, Task>,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct PueueGroup {
    pub parallel_tasks: u16,
    pub status: String,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct PueueTaskLog {
    pub output: String,
    pub task: Task,
}