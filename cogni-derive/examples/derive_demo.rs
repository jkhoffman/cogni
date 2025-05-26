//! Example demonstrating the StructuredOutput derive macro

use cogni_core::StructuredOutput;
use cogni_derive::StructuredOutput;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct Person {
    name: String,
    age: u32,
    email: Option<String>,
    tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct Address {
    street: String,
    city: String,
    state: String,
    zip: String,
    country: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
struct Company {
    name: String,
    founded: u32,
    employees: Vec<Person>,
    headquarters: Address,
    revenue: Option<f64>,
    public: bool,
}

fn main() {
    // Demonstrate the generated schemas
    println!("Person Schema:");
    println!(
        "{}",
        serde_json::to_string_pretty(&Person::schema()).unwrap()
    );

    println!("\nAddress Schema:");
    println!(
        "{}",
        serde_json::to_string_pretty(&Address::schema()).unwrap()
    );

    println!("\nCompany Schema:");
    println!(
        "{}",
        serde_json::to_string_pretty(&Company::schema()).unwrap()
    );
}
