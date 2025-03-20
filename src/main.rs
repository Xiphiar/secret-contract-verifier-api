#[macro_use]
extern crate rocket;

use std::{collections::HashMap, env, process::Command};

use displayable::{StatusDisplayable, TaskDisplayable};
use pueue::{PueueStatus, PueueTaskLog};
use rocket::form::{self, Error, Form, FromForm};
use url::Url;

use serde::{Deserialize, Serialize};

mod displayable;
mod pueue;

fn validate_commit<'v>(commit: &str) -> form::Result<'v, ()> {
    if commit == "HEAD" || commit.to_lowercase() == "main" || commit.to_lowercase() == "master" {
        return Ok(());
    }
    if commit.len() < 4 {
        return Err(Error::validation(
            "Commit must be at least 4 characters long".to_string(),
        ))?;
    }
    if commit.len() > 40 {
        return Err(Error::validation(
            "Commit must be at most 40 characters long".to_string(),
        ))?;
    }
    // if !commit.chars().all(|c| c.is_ascii_hexdigit()) {
    //     return Err(Error::validation(
    //         "Commit must only contain hexdigits".to_string(),
    //     ))?;
    // }
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

fn validate_optimizer<'v>(optimizer: &str) -> form::Result<'v, ()> {
    // Check if optimizer version follows semantic versioning pattern (e.g., 1.0.10)
    let parts: Vec<&str> = optimizer.split('.').collect();
    
    if parts.len() != 3 {
        return Err(Error::validation(
            "Optimizer version must be in format x.y.z".to_string(),
        ))?;
    }
    
    // Validate each part is a number
    for part in parts {
        if !part.chars().all(|c| c.is_ascii_digit()) {
            return Err(Error::validation(
                "Optimizer version must only contain numbers and dots".to_string(),
            ))?;
        }
    }
    
    Ok(())
}

#[derive(Serialize, Deserialize, Debug, FromForm)]
struct EnqueueTask {
    #[field(default = None, validate = validate_repo())]
    repo: String,
    #[field(default = "HEAD", validate = validate_commit())]
    commit: String,
    #[field(name = "optimizer", default = Option::<String>::None)]
    optimizer: Option<String>,
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
    let status: PueueStatus = serde_json::from_str(stdout_str).unwrap();
    let tasks = status.tasks;
    let mut tasks_displayable: Vec<TaskDisplayable> = vec![];
    for (_, task) in tasks {
        tasks_displayable.push(TaskDisplayable {
            id: task.id,
            command: task.command,
            status: task.status,
            created_at: task.created_at,
            start: task.start,
            end: task.end,
            output: None,
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
    let tasks: HashMap<String, PueueTaskLog> = serde_json::from_str(out_str).unwrap();
    let task_log = tasks.get(&id.to_string()).unwrap();
    let task = task_log.task.clone();
    let task_displayable = TaskDisplayable {
        id: task.id,
        command: task.command.clone(),
        status: task.status.clone(),
        created_at: task.created_at,
        start: task.start,
        end: task.end,
        output: Some(task_log.output.clone()),
    };
    serde_json::to_string(&task_displayable).unwrap()
}

#[post("/enqueue", data = "<task>")]
fn enqueue(task: Form<EnqueueTask>) -> String {
    let mut command = Command::new("pueue");
    command
        .arg("add")
        .arg("--print-task-id")
        .arg("--")
        .arg("secret-contract-verifier")
        .arg("--repo")
        .arg(task.repo.clone())
        .arg("--commit")
        .arg(task.commit.clone());
    
    // Add optimizer argument if provided
    if let Some(optimizer) = &task.optimizer {
        // Validate optimizer version before using it
        match validate_optimizer(optimizer) {
            Ok(_) => command.arg("--optimizer").arg(optimizer),
            Err(_) => return format!("Invalid optimizer version: must be in format x.y.z with numeric values only"),
        };
    }
    
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
