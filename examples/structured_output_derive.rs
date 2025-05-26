//! Example demonstrating the StructuredOutput derive macro

use cogni::prelude::*;
use cogni::providers::OpenAI;
use cogni::{ResponseFormat, StructuredOutput};
use serde::{Deserialize, Serialize};

// Define a struct with the StructuredOutput derive macro
#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct WeatherReport {
    location: String,
    temperature: f64,
    conditions: String,
    humidity: Option<u32>,
    wind_speed: Option<f64>,
    forecast: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct Person {
    name: String,
    age: u32,
    email: Option<String>,
    phone: Option<String>,
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
            if let Some(humidity) = weather.humidity {
                println!("Humidity: {}%", humidity);
            }
            if let Some(wind) = weather.wind_speed {
                println!("Wind Speed: {} mph", wind);
            }
            if !weather.forecast.is_empty() {
                println!("Forecast: {:?}", weather.forecast);
            }
        }
    } else {
        println!("Set OPENAI_API_KEY to test with real API");
    }

    Ok(())
}
