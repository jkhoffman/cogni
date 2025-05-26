//! Example of using structured output with various providers
//!
//! This example shows how to use the structured output feature to get
//! well-defined JSON responses from LLMs.

use cogni::prelude::*;
use cogni::{
    providers::{Anthropic, OpenAI},
    StructuredOutput,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::error::Error;

/// A weather report structure that we want the LLM to fill
#[derive(Debug, Serialize, Deserialize)]
struct WeatherReport {
    location: String,
    temperature: f32,
    conditions: String,
    humidity: u8,
    wind_speed: f32,
}

impl StructuredOutput for WeatherReport {
    fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The location of the weather report"
                },
                "temperature": {
                    "type": "number",
                    "description": "Temperature in Fahrenheit"
                },
                "conditions": {
                    "type": "string",
                    "description": "Weather conditions (e.g., sunny, cloudy, rainy)"
                },
                "humidity": {
                    "type": "integer",
                    "minimum": 0,
                    "maximum": 100,
                    "description": "Relative humidity percentage"
                },
                "wind_speed": {
                    "type": "number",
                    "description": "Wind speed in miles per hour"
                }
            },
            "required": ["location", "temperature", "conditions", "humidity", "wind_speed"],
            "additionalProperties": false
        })
    }
}

/// A person structure for extraction
#[derive(Debug, Serialize, Deserialize)]
struct Person {
    name: String,
    age: Option<u8>,
    occupation: Option<String>,
    skills: Vec<String>,
}

impl StructuredOutput for Person {
    fn schema() -> serde_json::Value {
        json!({
            "type": "object",
            "properties": {
                "name": {
                    "type": "string",
                    "description": "The person's full name"
                },
                "age": {
                    "type": ["integer", "null"],
                    "minimum": 0,
                    "maximum": 150,
                    "description": "The person's age in years"
                },
                "occupation": {
                    "type": ["string", "null"],
                    "description": "The person's job or profession"
                },
                "skills": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "List of the person's skills"
                }
            },
            "required": ["name", "skills"],
            "additionalProperties": false
        })
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Example 1: Using structured output with OpenAI
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        println!("=== OpenAI Structured Output Example ===\n");

        let provider = OpenAI::with_api_key(api_key);
        let client = Client::new(provider);

        // Request weather information
        let weather: WeatherReport = client
            .chat_structured(
                "What's the current weather like in San Francisco? Please provide realistic data.",
            )
            .await?;

        println!("Weather Report:");
        println!("  Location: {}", weather.location);
        println!("  Temperature: {}°F", weather.temperature);
        println!("  Conditions: {}", weather.conditions);
        println!("  Humidity: {}%", weather.humidity);
        println!("  Wind Speed: {} mph", weather.wind_speed);
        println!();
    }

    // Example 2: Using JSON mode without a specific schema
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        println!("=== JSON Mode Example ===\n");

        let provider = OpenAI::with_api_key(api_key);
        let client = Client::new(provider);

        let response = client
            .request()
            .system("You are a helpful assistant that always responds in JSON format.")
            .user("List three programming languages with their strengths")
            .json_mode()
            .send()
            .await?;

        let json_value = response.parse_json()?;
        println!(
            "JSON Response: {}",
            serde_json::to_string_pretty(&json_value)?
        );
        println!();
    }

    // Example 3: Extracting structured data from text
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        println!("=== Data Extraction Example ===\n");

        let provider = OpenAI::with_api_key(api_key);
        let client = Client::new(provider);

        let text = "John Doe is a 35-year-old software engineer who specializes in \
                    Rust, Python, and machine learning. He also has experience with \
                    cloud computing and database design.";

        let person: Person = client
            .request()
            .system("Extract person information from the provided text.")
            .user(text)
            .with_structured_output::<Person>()
            .send()
            .await?
            .parse_structured()?;

        println!("Extracted Person:");
        println!("  Name: {}", person.name);
        println!("  Age: {:?}", person.age);
        println!("  Occupation: {:?}", person.occupation);
        println!("  Skills: {:?}", person.skills);
        println!();
    }

    // Example 4: Using structured output with Anthropic (if supported)
    if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
        println!("=== Anthropic Structured Output Example ===\n");

        let provider = Anthropic::with_api_key(api_key);
        let client = Client::new(provider);

        // Note: Anthropic's structured output support may vary
        let response = client
            .request()
            .system("You are a helpful assistant. Please respond with a JSON object containing weather information.")
            .user("What's the weather like in New York? Respond in JSON format.")
            .json_mode()
            .model("claude-3-opus-20240229")
            .send()
            .await?;

        if let Ok(json) = response.parse_json() {
            println!(
                "Anthropic JSON Response: {}",
                serde_json::to_string_pretty(&json)?
            );
        } else {
            println!("Response: {}", response.content);
        }
    }

    Ok(())
}
