//! Example demonstrating the StructuredOutput derive macro

use cogni::prelude::*;
use cogni::providers::{Ollama, OpenAI};
use cogni::{ResponseFormat, StructuredOutput};
use serde::{Deserialize, Serialize};

// Define a struct with the StructuredOutput derive macro
// Note: When using strict mode with OpenAI, all fields must be required (non-optional)
#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct WeatherReport {
    location: String,
    temperature: f64,
    conditions: String,
    humidity: u32,
    wind_speed: f64,
    forecast: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct Person {
    name: String,
    age: u32,
    email: String,
    phone: String,
    interests: Vec<String>,
}

#[tokio::main]
async fn main() -> Result<(), cogni::Error> {
    // Show the generated schemas
    println!("WeatherReport Schema:");
    println!(
        "{}\n",
        serde_json::to_string_pretty(&WeatherReport::schema()).unwrap()
    );

    println!("Person Schema:");
    println!(
        "{}\n",
        serde_json::to_string_pretty(&Person::schema()).unwrap()
    );

    // Example usage with OpenAI (requires API key)
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        let provider = OpenAI::with_api_key(api_key);

        // Build request with structured output
        let request = Request::builder()
            .message(Message::system("You are a weather service API."))
            .message(Message::user("What's the weather like in San Francisco?"))
            .model("gpt-4o")
            .response_format(ResponseFormat::JsonSchema {
                schema: WeatherReport::schema(),
                strict: true,
            })
            .build();

        println!("Sending request for structured weather data...");
        let response = provider.request(request).await?;

        // Parse the structured response
        if let Ok(weather) = serde_json::from_str::<WeatherReport>(&response.content) {
            println!("\nWeather Report:");
            println!("Location: {}", weather.location);
            println!("Temperature: {}°F", weather.temperature);
            println!("Conditions: {}", weather.conditions);
            println!("Humidity: {}%", weather.humidity);
            println!("Wind Speed: {} mph", weather.wind_speed);
            if !weather.forecast.is_empty() {
                println!("Forecast: {:?}", weather.forecast);
            }
        }
    } else {
        println!("Set OPENAI_API_KEY to test with real API");
    }

    // Test with Anthropic (requires API key)
    if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
        use cogni::providers::Anthropic;

        println!("\n\n=== Testing with Anthropic ===");

        let provider = Anthropic::with_api_key(api_key);

        // Build request with structured output
        let request = Request::builder()
            .message(Message::system("You are a weather service API."))
            .message(Message::user("What's the weather like in Paris?"))
            .model("claude-3-haiku-20240307")
            .response_format(ResponseFormat::JsonSchema {
                schema: WeatherReport::schema(),
                strict: true,
            })
            .build();

        println!("Sending request for structured weather data...");
        let response = provider.request(request).await?;

        // Parse the structured response
        if let Ok(weather) = serde_json::from_str::<WeatherReport>(&response.content) {
            println!("\nWeather Report:");
            println!("Location: {}", weather.location);
            println!("Temperature: {}°F", weather.temperature);
            println!("Conditions: {}", weather.conditions);
            println!("Humidity: {}%", weather.humidity);
            println!("Wind Speed: {} mph", weather.wind_speed);
            if !weather.forecast.is_empty() {
                println!("Forecast: {:?}", weather.forecast);
            }
        }
    }

    // Test with Ollama
    println!("\n\n=== Testing with Ollama ===");

    let provider = Ollama::local();

    // Build request with structured output
    let request = Request::builder()
        .message(Message::system("You are a weather service API."))
        .message(Message::user("What's the weather like in London?"))
        .model("llama3.2")
        .response_format(ResponseFormat::JsonSchema {
            schema: WeatherReport::schema(),
            strict: true,
        })
        .build();

    println!("Sending request for structured weather data...");
    match provider.request(request).await {
        Ok(response) => {
            // Parse the structured response
            if let Ok(weather) = serde_json::from_str::<WeatherReport>(&response.content) {
                println!("\nWeather Report:");
                println!("Location: {}", weather.location);
                println!("Temperature: {}°F", weather.temperature);
                println!("Conditions: {}", weather.conditions);
                println!("Humidity: {}%", weather.humidity);
                println!("Wind Speed: {} mph", weather.wind_speed);
                if !weather.forecast.is_empty() {
                    println!("Forecast: {:?}", weather.forecast);
                }
            } else {
                println!("Failed to parse response: {}", response.content);
            }
        }
        Err(e) => {
            println!("Ollama request failed (is Ollama running?): {}", e);
        }
    }

    Ok(())
}
