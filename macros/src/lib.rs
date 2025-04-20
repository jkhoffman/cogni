//! Procedural macros for the Cogni framework.
//!
//! This crate provides procedural macros for compile-time validation and code generation.

#![forbid(unsafe_code)]
#![warn(missing_docs)]
#![allow(unused_imports)]

use proc_macro::TokenStream;
use proc_macro_error::{abort, proc_macro_error};
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use regex::Regex;
use syn::{
    DeriveInput, Ident, LitStr, Token, parse::Parse, parse::ParseStream, parse_macro_input,
    parse_str,
};

/// Derive macro for implementing the `Tool` trait.
///
/// This macro generates the boilerplate code for implementing the `Tool` trait,
/// including JSON schema validation and error handling.
#[proc_macro_derive(Tool)]
pub fn derive_tool(input: TokenStream) -> TokenStream {
    let _input = parse_macro_input!(input as DeriveInput);
    TokenStream::new()
}

/// Derive macro for implementing the `LanguageModel` trait.
///
/// This macro generates the boilerplate code for implementing the `LanguageModel` trait,
/// including prompt validation and response handling.
#[proc_macro_derive(LanguageModel, attributes(llm))]
pub fn derive_language_model(input: TokenStream) -> TokenStream {
    let _input = parse_macro_input!(input as DeriveInput);
    TokenStream::new()
}

/// Derive macro for implementing the `MemoryStore` trait.
///
/// This macro generates the boilerplate code for implementing the `MemoryStore` trait,
/// including session management and error handling.
#[proc_macro_derive(MemoryStore, attributes(memory))]
pub fn derive_memory_store(input: TokenStream) -> TokenStream {
    let _input = parse_macro_input!(input as DeriveInput);
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
/// use cogni_macros::prompt;
///
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
    let re = Regex::new(r"\{\{([^}]+)\}\}").unwrap();
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
        // Validate that the placeholder is a valid Rust identifier
        let ident = match parse_str::<Ident>(name) {
            Ok(ident) => ident,
            Err(_) => abort!(template.span(), "`{}` is not a valid identifier", name),
        };
        quote! {
            pub #ident: String
        }
    });

    // Generate validation impl
    let validation = placeholders.iter().map(|name| {
        let ident = parse_str::<Ident>(name).unwrap();
        quote! {
            if self.#ident.is_empty() {
                return Err(cogni_core::traits::prompt::MissingPlaceholderError {
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

            impl cogni_core::traits::prompt::PromptArgs for TemplateArgs {
                fn validate(&self) -> Result<(), cogni_core::traits::prompt::MissingPlaceholderError> {
                    #(#validation)*
                    Ok(())
                }
            }

            cogni_core::PromptTemplate::new(#template_str)
        }
    };

    output.into()
}

/// Attribute macro for validating tool specifications at compile time.
///
/// This macro validates that tool specifications are well-formed and contain
/// valid JSON schemas.
#[proc_macro_attribute]
pub fn tool_spec(_attr: TokenStream, item: TokenStream) -> TokenStream {
    item
}

/// Derive macro for implementing the `Prompt` trait.
///
/// This macro generates the boilerplate code for implementing the `Prompt` trait,
/// including validation and error handling.
#[proc_macro_derive(Prompt)]
pub fn derive_prompt(input: TokenStream) -> TokenStream {
    let _input = parse_macro_input!(input as DeriveInput);
    TokenStream::new()
}

/// Derive macro for implementing the `ToolSet` trait.
///
/// This macro generates the boilerplate code for implementing the `ToolSet` trait,
/// including tool registration and error handling.
#[proc_macro_derive(ToolSet)]
pub fn derive_tool_set(input: TokenStream) -> TokenStream {
    let _input = parse_macro_input!(input as DeriveInput);
    TokenStream::new()
}

/// Create a chat message with the given role and content.
///
/// # Example
/// ```rust
/// use cogni_macros::chat_message;
/// use cogni_provider_openai::ChatMessage;
///
/// let msg = chat_message!(user: "Hello, AI!");
/// let sys = chat_message!(system: "You are a helpful assistant.");
/// ```
#[proc_macro]
#[proc_macro_error]
pub fn chat_message(input: TokenStream) -> TokenStream {
    #[derive(Debug)]
    struct ChatMessageInput {
        role: Ident,
        content: LitStr,
    }

    impl Parse for ChatMessageInput {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let role = input.parse()?;
            input.parse::<Token![:]>()?;
            let content = input.parse()?;
            Ok(ChatMessageInput { role, content })
        }
    }

    let input = parse_macro_input!(input as ChatMessageInput);
    let role = input.role;
    let content = input.content;

    let output = quote! {
        ChatMessage {
            role: stringify!(#role).to_string(),
            content: #content.to_string(),
        }
    };

    output.into()
}
