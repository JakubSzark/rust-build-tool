use std::fs::File;
use std::process::Command;
use std::{
    collections::{HashMap, VecDeque},
    process::Output,
};

const BUILD_CONFIG: &str = "build.cfg";

struct Task {
    name: String,
    command: String,
}

/// Opens or creates the build config. Then
/// returns the contents as a `String`
fn get_build_config() -> Result<String, &'static str> {
    use std::io::prelude::*;

    // First we try to open the build config
    let file = File::open(BUILD_CONFIG);

    // If it doesn't exist then just create it.
    if file.is_err() {
        if File::create(BUILD_CONFIG).is_err() {
            return Err("failed to create build config!");
        }

        println!("info: {} created!", BUILD_CONFIG);
        return Ok(String::new());
    }

    // Here we read the contents into a String
    let mut result = String::new();
    if file.unwrap().read_to_string(&mut result).is_err() {
        return Err("failed to read build config!");
    }

    return Ok(result);
}

/// Splits a line by an = and reads it as a key and value pair
fn get_line_key_value<'a>(line: &'a str) -> Option<(&'a str, &'a str)> {
    let trim_pat = |c| c == ' ' || c == '\"';
    let mut split = line.split("=");

    // Both key and value must exist to be added

    let key = match split.next() {
        Some(key) => key.trim_matches(trim_pat),
        None => return None,
    };

    let value = match split.next() {
        Some(value) => value.trim_matches(trim_pat),
        None => return None,
    };

    Some((key, value))
}

/// This extract the variables from the config file.
/// Variables start with a $ and are assigned with an =
fn get_variables_map(config: &String) -> HashMap<String, String> {
    let lines = config.lines();
    let mut variables = HashMap::new();

    for line in lines {
        let trimmed = line.trim();

        // We need to make sure the line starts with a $
        if let Some(first) = trimmed.chars().nth(0) {
            if first != '$' {
                continue;
            }
        }

        if let Some((key, value)) = get_line_key_value(trimmed) {
            variables.insert(String::from(key), String::from(value));
        }
    }

    return variables;
}

/// Extracts the tasks from config file.
fn get_user_tasks(config: &String) -> Vec<Task> {
    let lines = config.lines();
    let mut tasks = Vec::new();
    let mut task_name = String::new();
    let mut task_found = false;

    for line in lines {
        // Ignore empty lines
        if line.is_empty() {
            continue;
        }

        let trimmed = line.trim();

        // We look for task headers first
        if !task_found {
            // Task headers start with an open bracket
            if let Some(first) = trimmed.chars().nth(0) {
                if first != '[' {
                    continue;
                }
            }

            // Also the header ends with a close bracket
            if let Some(last) = trimmed.chars().last() {
                if (last) != ']' {
                    continue;
                }
            }

            let trim_pat = |c| c == '[' || c == ']';
            task_name.push_str(trimmed.trim_matches(trim_pat));
            task_found = true;
        } else {
            // Tasks have a command
            if !trimmed.starts_with("command") {
                continue;
            }

            if let Some((_, value)) = get_line_key_value(trimmed) {
                if value.is_empty() {
                    println!("warn: task({}) has no command", task_name);
                    continue;
                }

                tasks.push(Task {
                    name: task_name.clone(),
                    command: String::from(value),
                });

                task_name.clear();
                task_found = false;
            }
        }
    }

    return tasks;
}

/// This retrieves the execution task queue from the config file.
fn get_execute_queue(config: &String) -> VecDeque<String> {
    let lines = config.lines();
    let mut queue = VecDeque::new();
    let mut in_execute_task = false;

    for line in lines {
        if line.starts_with("[execute]") {
            in_execute_task = true;
            continue;
        }

        if in_execute_task && line.is_empty() {
            break;
        }

        if in_execute_task {
            queue.push_back(String::from(line));
        }
    }

    return queue;
}

fn output_task_result(task_name: &String, output: Output) {
    if output.status.success() {
        println!("\rtask({}): finished", task_name);
        if let Ok(stdout) = String::from_utf8(output.stdout) {
            if !stdout.is_empty() {
                println!("\n{}", stdout);
            }
        }
    } else {
        println!("\rtask({}): failed", task_name);
        if let Ok(stderr) = String::from_utf8(output.stderr) {
            if !stderr.is_empty() {
                println!("\n{}", stderr);
            }
        }
    }
}

fn main() {
    println!("info: reading {}...", BUILD_CONFIG);

    let config = match get_build_config() {
        Ok(config) => config,
        Err(e) => {
            println!("error: {}", e);
            return;
        }
    };

    let variables = get_variables_map(&config);
    let tasks = get_user_tasks(&config);

    println!(
        "info: found {} var(s) and {} task(s)",
        variables.len(),
        tasks.len()
    );

    let mut commands = HashMap::new();

    // Replace variables in a task's command
    for task in tasks {
        let mut split = task.command.split(" ");

        if let Some(first) = split.nth(0) {
            let mut command = Command::new(first);

            while let Some(arg) = split.next() {
                match variables.get(arg) {
                    Some(val) => command.arg(val),
                    None => command.arg(arg),
                };
            }

            commands.insert(task.name, command);
        }
    }

    let mut queue = get_execute_queue(&config);

    if queue.is_empty() {
        println!("info: execute task is empty");
        return;
    }

    while let Some(task_name) = queue.pop_front() {
        if let Some(command) = commands.get_mut(&task_name) {
            print!("task({}): started", task_name);
            match command.output() {
                Err(e) => {
                    println!("\rtask({}): failed to execute\n{}", task_name, e);
                }
                Ok(output) => output_task_result(&task_name, output),
            }
        }
    }
}
