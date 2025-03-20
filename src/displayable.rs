use chrono::{DateTime, Local};
use pueue_lib::task::TaskStatus;
use serde::{Deserialize, Serialize};

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub struct TaskDisplayable {
    pub id: usize,
    pub command: String,
    pub status: TaskStatus,
    pub created_at: DateTime<Local>,
    pub start: Option<DateTime<Local>>,
    pub end: Option<DateTime<Local>>,
    pub output: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StatusDisplayable {
    pub tasks: Vec<TaskDisplayable>,
}

// #[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
// pub enum TaskResult {
//     Success,
//     Failed(i32),
//     FailedToSpawn(String),
//     Killed,
//     Errored,
//     DependencyFailed,
// }