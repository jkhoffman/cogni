//! Procedural macros for the Cogni framework.
//!
//! This crate provides procedural macros for compile-time validation and code generation.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use quote::quote;
use quote::{ToTokens, quote};
use regex;
use syn::{DeriveInput, parse_macro_input};
use syn::{LitStr, parse_macro_input};

/// Derive macro for implementing the `Tool` trait.
///
/// This macro generates the boilerplate code for implementing the `Tool` trait,
/// including JSON schema validation and error handling.
#[proc_macro_derive(Tool, attributes(tool))]
pub fn derive_tool(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    // TODO: Implement tool derive macro
    TokenStream::new()
}

/// Derive macro for implementing the `LanguageModel` trait.
///
/// This macro generates the boilerplate code for implementing the `LanguageModel` trait,
/// including prompt validation and response handling.
#[proc_macro_derive(LanguageModel, attributes(llm))]
pub fn derive_language_model(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    // TODO: Implement language model derive macro
    TokenStream::new()
}

/// Derive macro for implementing the `MemoryStore` trait.
///
/// This macro generates the boilerplate code for implementing the `MemoryStore` trait,
/// including session management and error handling.
#[proc_macro_derive(MemoryStore, attributes(memory))]
pub fn derive_memory_store(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    // TODO: Implement memory store derive macro
    TokenStream::new()
}

/// Compile-time validated prompt template.
///
/// This macro takes a string literal containing a template with {{placeholder}} syntax
/// and generates a struct containing the required fields. The struct implements
/// PromptArgs automatically.
///
/// # Example
///
/// ```rust
/// let template = prompt!("Hello {{name}}, you are {{age}} years old!");
/// // Generates:
/// // struct TemplateArgs {
/// //     name: String,
/// //     age: String,
/// // }
/// ```
#[proc_macro]
#[proc_macro_error]
pub fn prompt(input: TokenStream) -> TokenStream {
    let template = parse_macro_input!(input as LitStr);
    let template_str = template.value();

    // Extract placeholders using regex
    let re = regex::Regex::new(r"\{\{([^}]+)\}\}").unwrap();
    let placeholders: Vec<String> = re
        .captures_iter(&template_str)
        .map(|cap| cap[1].trim().to_string())
        .collect();

    if placeholders.is_empty() {
        abort!(
            template.span(),
            "prompt template must contain at least one placeholder"
        );
    }

    // Generate the struct fields
    let fields = placeholders.iter().map(|name| {
        let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
        quote! {
            pub #ident: String
        }
    });

    // Generate validation impl
    let validation = placeholders.iter().map(|name| {
        let ident = syn::Ident::new(name, proc_macro2::Span::call_site());
        quote! {
            if self.#ident.is_empty() {
                return Err(MissingPlaceholderError {
                    name: stringify!(#ident).to_string()
                });
            }
        }
    });

    let output = quote! {
        {
            #[derive(Debug, serde::Serialize)]
            pub struct TemplateArgs {
                #(#fields,)*
            }

            impl cogni_core::prompt::PromptArgs for TemplateArgs {
                fn validate(&self) -> Result<(), cogni_core::prompt::MissingPlaceholderError> {
                    #(#validation)*
                    Ok(())
                }
            }

            cogni_core::prompt::PromptTemplate::new(#template_str)
        }
    };

    output.into()
}

/// Attribute macro for validating tool specifications at compile time.
///
/// This macro validates that tool specifications are well-formed and contain
/// valid JSON schemas.
#[proc_macro_attribute]
pub fn tool_spec(attr: TokenStream, item: TokenStream) -> TokenStream {
    // TODO: Implement tool spec validation macro
    item
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_prompt_macro() {
        let input = "Hello {{name}}, you are {{age}} years old!"
            .parse()
            .unwrap();
        let output = prompt(input);
        println!("{}", output);
    }
}
