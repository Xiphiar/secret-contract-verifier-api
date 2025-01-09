#[macro_use]
extern crate rocket;

use std::{collections::HashMap, env, process::Command};

use chrono::prelude::*;
use rocket::form::{self, Error, Form, FromForm};
use url::Url;

use serde::{Deserialize, Serialize};

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum TaskStatus {
    Queued,
    Stashed { enqueue_at: Option<DateTime<Local>> },
    Running,
    Paused,
    Done(TaskResult),
    Locked,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
pub enum TaskResult {
    Success,
    Failed(i32),
    FailedToSpawn(String),
    Killed,
    Errored,
    DependencyFailed,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
struct Task {
    id: u32,
    original_command: String,
    command: String,
    path: String,
    envs: HashMap<String, String>,
    group: String,
    dependencies: Vec<u32>,
    label: Option<String>,
    status: TaskStatus,
    prev_status: String,
    start: Option<DateTime<Local>>,
    end: Option<DateTime<Local>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Status {
    tasks: HashMap<String, Task>,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
struct TaskDisplayable {
    id: u32,
    command: String,
    status: TaskStatus,
    prev_status: String,
    start: Option<DateTime<Local>>,
    end: Option<DateTime<Local>>,
}

#[derive(Serialize, Deserialize, Debug)]
struct StatusDisplayable {
    tasks: Vec<TaskDisplayable>,
}

fn validate_commit<'v>(commit: &str) -> form::Result<'v, ()> {
    if commit == "HEAD" {
        return Ok(());
    }
    if commit.len() < 7 {
        return Err(Error::validation(
            "Commit must be at least 7 characters long".to_string(),
        ))?;
    }
    if commit.len() > 40 {
        return Err(Error::validation(
            "Commit must be at most 40 characters long".to_string(),
        ))?;
    }
    if !commit.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(Error::validation(
            "Commit must only contain hexdigits".to_string(),
        ))?;
    }
    Ok(())
}

fn validate_repo<'v>(repo: &str) -> form::Result<'v, ()> {
    if !(repo.starts_with("git@") || repo.starts_with("https://")) {
        return Err(Error::validation(
            "Repository must start with git@ or https://".to_string(),
        ))?;
    }
    if repo.starts_with("https://") && !repo.ends_with(".git") {
        return Err(Error::validation(
            "Repository must end with .git".to_string(),
        ))?;
    }
    if !repo.chars().all(|c| {
        c.is_ascii_alphanumeric()
            || c == '.'
            || c == '-'
            || c == '_'
            || c == '@'
            || c == ':'
            || c == '/'
    }) {
        return Err(Error::validation(
            "Repository must only contain alphanumeric characters, ., -, _, @, : or /".to_string(),
        ))?;
    }
    if repo.contains("..") {
        return Err(Error::validation(
            "Repository must not contain ..".to_string(),
        ))?;
    }
    let parsed = Url::parse(repo);
    if parsed.is_err() {
        return Err(Error::validation(
            "Repository must be a valid URL".to_string(),
        ))?;
    }

    Ok(())
}

#[derive(Serialize, Deserialize, Debug, FromForm)]
struct EnqueueTask {
    #[field(default = None)]
    code_id: u16,
    #[field(default = None, validate = validate_repo())]
    repo: String,
    #[field(default = "HEAD", validate = validate_commit())]
    commit: String,
    #[field(default = None)]
    chain_id: String,
    #[field(default = None)]
    lcd: String,
}

#[derive(PartialEq, Clone, Debug, Serialize, Deserialize)]
struct TaskLog {
    output: String,
    task: Task,
}

#[get("/status")]
fn get_status() -> String {
    let mut command = Command::new("pueue");
    command.arg("status").arg("--json");
    let out = command.output().unwrap();
    let stdout = out.stdout;
    let stderr = out.stderr;
    println!("{}", std::str::from_utf8(&stderr).unwrap());
    let stdout_str = std::str::from_utf8(&stdout).unwrap();
    let status: Status = serde_json::from_str(stdout_str).unwrap();
    let tasks = status.tasks;
    let mut tasks_displayable: Vec<TaskDisplayable> = vec![];
    for (_, task) in tasks {
        tasks_displayable.push(TaskDisplayable {
            id: task.id,
            command: task.command,
            status: task.status,
            prev_status: task.prev_status,
            start: task.start,
            end: task.end,
        });
    }
    tasks_displayable.sort_by(|a, b| a.id.cmp(&b.id));
    let status_displayable = StatusDisplayable {
        tasks: tasks_displayable,
    };
    serde_json::to_string(&status_displayable).unwrap()
}

#[get("/status/<id>")]
fn get_status_for_id(id: u32) -> String {
    let mut command = Command::new("pueue");
    command.arg("log").arg(id.to_string()).arg("--json");
    let out = command.output().unwrap().stdout;
    let out_str = std::str::from_utf8(&out).unwrap();
    let tasks: HashMap<String, TaskLog> = serde_json::from_str(out_str).unwrap();
    let task = tasks.get(&id.to_string()).unwrap().task.clone();
    let task_displayable = TaskDisplayable {
        id: task.id,
        command: task.command.clone(),
        status: task.status.clone(),
        prev_status: task.prev_status.clone(),
        start: task.start,
        end: task.end,
    };
    serde_json::to_string(&task_displayable).unwrap()
}

#[post("/enqueue", data = "<task>")]
fn enqueue(task: Form<EnqueueTask>) -> String {
    let mut command = Command::new("pueue");
    command
        .arg("add")
        .arg("--")
        .arg("secret-contract-verifier")
        .arg("--repo")
        .arg(task.repo.clone())
        .arg("--commit")
        .arg(task.commit.clone())
        .arg("--code-id")
        .arg(task.code_id.to_string())
        .arg("--chain-id")
        .arg(task.chain_id.to_string())
        .arg("--lcd")
        .arg(task.lcd.to_string());
        // .arg("--require-sudo")
        // .arg("--database_contract")
        // .arg(env::var("MONGODB_URI").unwrap());
    let out = command.output().unwrap().stdout;
    format!("{}", std::str::from_utf8(&out).unwrap())
}

#[get("/")]
fn root() -> String {
    format!(
        "
    /status - Get status of all tasks
    /status/<id> - Get status of a task
    /enqueue/<id> - Enqueue a task
        "
    )
}

#[launch]
fn rocket() -> _ {
    let mongo_uri = env::var("MONGODB_URI");
    if mongo_uri.is_err() {
        panic!("Environment variable MONGODB_URI is not set!")
    }
    rocket::build().mount("/", routes![root, get_status, get_status_for_id, enqueue])
}
