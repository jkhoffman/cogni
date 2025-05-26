//! Example showing how to handle structured output with fallback for unsupported models
//!
//! This example demonstrates:
//! - Checking if a model supports structured output
//! - Falling back to JSON mode or manual parsing
//! - Error handling for structured output failures

use cogni::prelude::*;
use cogni::{providers::OpenAI, StructuredOutput};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error;

#[derive(Debug, Serialize, Deserialize)]
struct Task {
    title: String,
    description: String,
    priority: String,
    estimated_hours: f32,
}

impl StructuredOutput for Task {
    fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "title": {
                    "type": "string",
                    "description": "Brief title of the task"
                },
                "description": {
                    "type": "string",
                    "description": "Detailed description of what needs to be done"
                },
                "priority": {
                    "type": "string",
                    "enum": ["low", "medium", "high"],
                    "description": "Task priority level"
                },
                "estimated_hours": {
                    "type": "number",
                    "description": "Estimated hours to complete the task"
                }
            },
            "required": ["title", "description", "priority", "estimated_hours"],
            "additionalProperties": false
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let api_key = std::env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY must be set");

    println!("=== Structured Output with Fallback Example ===\n");

    // Example 1: Try with a model that supports structured output
    println!("1. Attempting with gpt-4o (supports structured output):");
    match try_structured_output(&api_key, "gpt-4o").await {
        Ok(task) => {
            println!("✅ Success with structured output!");
            print_task(&task);
        }
        Err(e) => {
            println!("❌ Failed: {}", e);
        }
    }

    println!("\n2. Attempting with gpt-3.5-turbo (may not support structured output):");
    match try_structured_output(&api_key, "gpt-3.5-turbo").await {
        Ok(task) => {
            println!("✅ Success with structured output!");
            print_task(&task);
        }
        Err(e) => {
            println!("❌ Structured output failed: {}", e);
            println!("   Falling back to JSON mode...");

            // Fallback to JSON mode
            match try_json_mode(&api_key, "gpt-3.5-turbo").await {
                Ok(task) => {
                    println!("✅ Success with JSON mode fallback!");
                    print_task(&task);
                }
                Err(e) => {
                    println!("❌ JSON mode also failed: {}", e);

                    // Final fallback: manual parsing
                    println!("   Falling back to manual parsing...");
                    match try_manual_parsing(&api_key, "gpt-3.5-turbo").await {
                        Ok(task) => {
                            println!("✅ Success with manual parsing!");
                            print_task(&task);
                        }
                        Err(e) => {
                            println!("❌ All methods failed: {}", e);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

async fn try_structured_output(api_key: &str, model: &str) -> Result<Task, Box<dyn Error>> {
    let provider = OpenAI::with_api_key(api_key);
    let client = Client::new(provider);

    let task: Task = client
        .request()
        .model(model)
        .system("You are a task creation assistant. Create tasks based on user input.")
        .user("Create a task for implementing user authentication in a web application")
        .with_structured_output::<Task>()
        .send()
        .await?
        .parse_structured()?;

    Ok(task)
}

async fn try_json_mode(api_key: &str, model: &str) -> Result<Task, Box<dyn Error>> {
    let provider = OpenAI::with_api_key(api_key);
    let client = Client::new(provider);

    let response = client
        .request()
        .model(model)
        .system(
            "You are a task creation assistant. Create tasks based on user input. \
             Always respond with valid JSON in this exact format: \
             {\"title\": \"...\", \"description\": \"...\", \"priority\": \"low|medium|high\", \"estimated_hours\": number}"
        )
        .user("Create a task for implementing user authentication in a web application")
        .json_mode()
        .send()
        .await?;

    let json_value = response.parse_json()?;
    let task: Task = serde_json::from_value(json_value)?;

    Ok(task)
}

async fn try_manual_parsing(api_key: &str, model: &str) -> Result<Task, Box<dyn Error>> {
    let provider = OpenAI::with_api_key(api_key);
    let client = Client::new(provider);

    let response = client
        .request()
        .model(model)
        .system(
            "You are a task creation assistant. Create tasks based on user input. \
             Format your response as follows:\n\
             Title: [task title]\n\
             Description: [detailed description]\n\
             Priority: [low/medium/high]\n\
             Estimated Hours: [number]",
        )
        .user("Create a task for implementing user authentication in a web application")
        .send()
        .await?;

    // Parse the response manually
    let content = response.content;
    let lines: Vec<&str> = content.lines().collect();

    let mut task = Task {
        title: String::new(),
        description: String::new(),
        priority: String::new(),
        estimated_hours: 0.0,
    };

    for line in lines {
        if let Some(title) = line.strip_prefix("Title: ") {
            task.title = title.trim().to_string();
        } else if let Some(desc) = line.strip_prefix("Description: ") {
            task.description = desc.trim().to_string();
        } else if let Some(priority) = line.strip_prefix("Priority: ") {
            task.priority = priority.trim().to_lowercase();
        } else if let Some(hours) = line.strip_prefix("Estimated Hours: ") {
            task.estimated_hours = hours.trim().parse().unwrap_or(0.0);
        }
    }

    // Validate
    if task.title.is_empty() || task.description.is_empty() {
        return Err("Failed to parse task from response".into());
    }

    Ok(task)
}

fn print_task(task: &Task) {
    println!("   Title: {}", task.title);
    println!("   Description: {}", task.description);
    println!("   Priority: {}", task.priority);
    println!("   Estimated Hours: {}", task.estimated_hours);
}
