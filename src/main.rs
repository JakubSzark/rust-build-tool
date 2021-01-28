use std::{collections::{HashMap, VecDeque}, io::Write};
use std::process::Command;
use std::fs::File;

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
        None => return None
    };

    let value = match split.next() {
        Some(value) => value.trim_matches(trim_pat),
        None => return None
    };

    Some((key, value))
}

/// This extract the variables from the config file.
/// Variables start with a $ and are assigned with an =
fn get_variables_map(config: &String) -> HashMap<String, String> {
    let lines = config.lines();
    let mut variables = HashMap::new();

    for line in lines {
        // We need to make sure the line starts with a $
        if let Some(first) = line.chars().nth(0) {
            if first != '$' {
                continue;
            }
        }

        if let Some((key, value)) = get_line_key_value(line) {
            variables.insert(String::from(key), String::from(value));
        }
    }

    return variables;
}

/// Extracts the tasks from config file.
fn get_user_tasks(config: &String) -> Vec<Task> {
    let lines = config.lines();
    let mut tasks = Vec::new();
    let mut name_found = false;

    let mut task_name = String::new();

    for line in lines {
        // Ignore empty lines
        if line.is_empty() {
            continue;
        }

        if !name_found {
            let trimmed = line.trim();

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
            task_name = String::from(trimmed.trim_matches(trim_pat));
            name_found = true;
        }
        else {
            // Tasks have a command
            if !line.starts_with("command") {
                continue;
            }

            if let Some((_, value)) = get_line_key_value(line) {
                tasks.push(Task {
                    name: task_name.clone(),
                    command: String::from(value)
                });

                name_found = false;
            } 
        }
    }

    return tasks;
}

fn carriage_return() {
    std::io::stdout().write("\r".as_bytes()).unwrap();
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

    println!("info: found {} var(s) and {} task(s)", 
        variables.len(), tasks.len());

    let mut commands = HashMap::new();

    // Replace variables in a task's command
    for task in tasks {
        let mut split = task.command.split(" ");

        if let Some(first) = split.nth(0) {
            let mut command = Command::new(first);

            while let Some(arg) = split.next() {
                if variables.contains_key(arg) {
                    command.arg(variables.get(arg).unwrap());
                } else {
                    command.arg(arg);
                }
            }

            commands.insert(task.name, command);
        }
    }

    let mut queue = get_execute_queue(&config);

    while let Some(task_name) = queue.pop_front() {
        if let Some(command) = commands.get_mut(&task_name) {
            print!("task({}): started", task_name);
            match command.output() {
                Err(e) => {
                    carriage_return();
                    println!("task({}): failed to execute task\n{}", task_name, e);
                },
                Ok(output) => {
                    if output.status.success() {
                        carriage_return();
                        println!("task({}): finished", task_name);
                        if let Ok(stdout) = String::from_utf8(output.stdout) {
                            if !stdout.is_empty() {
                                println!("\n{}", stdout);
                            }
                        }
                    } else {
                        carriage_return();
                        println!("task({}): Failed", task_name);
                        if let Ok(stderr) = String::from_utf8(output.stderr) {
                            if !stderr.is_empty() {
                                println!("\n{}", stderr);
                            }
                        }
                    }
                }
            }
        }
    }
}
