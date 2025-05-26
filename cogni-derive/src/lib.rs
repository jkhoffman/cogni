//! Derive macros for the Cogni framework

use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, Type};

/// Derive macro for the StructuredOutput trait
///
/// This macro automatically implements the `StructuredOutput` trait for structs,
/// generating a JSON Schema based on the struct's fields.
///
/// # Example
///
/// ```rust
/// use cogni_derive::StructuredOutput;
/// use serde::{Deserialize, Serialize};
///
/// #[derive(Debug, Clone, Serialize, Deserialize, StructuredOutput)]
/// struct Person {
///     name: String,
///     age: u32,
///     email: Option<String>,
/// }
/// ```
///
/// This will generate a JSON Schema that describes the Person struct.
#[proc_macro_derive(StructuredOutput)]
pub fn derive_structured_output(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Only support structs for now
    let fields = match &input.data {
        Data::Struct(data) => match &data.fields {
            Fields::Named(fields) => &fields.named,
            _ => {
                return syn::Error::new_spanned(
                    &input,
                    "StructuredOutput can only be derived for structs with named fields",
                )
                .to_compile_error()
                .into();
            }
        },
        _ => {
            return syn::Error::new_spanned(
                &input,
                "StructuredOutput can only be derived for structs",
            )
            .to_compile_error()
            .into();
        }
    };

    // Generate property definitions for each field
    let properties = fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let field_name_str = field_name.to_string();
        let field_type = &field.ty;

        let type_schema = generate_type_schema(field_type);

        quote! {
            #field_name_str: #type_schema
        }
    });

    // Generate required fields array
    let required_fields = fields.iter().filter_map(|field| {
        let field_name = field.ident.as_ref().unwrap().to_string();
        // Check if the field is Option<T>
        if is_option_type(&field.ty) {
            None
        } else {
            Some(quote! { #field_name })
        }
    });

    let expanded = quote! {
        impl cogni_core::StructuredOutput for #name {
            fn schema() -> serde_json::Value {
                serde_json::json!({
                    "type": "object",
                    "properties": {
                        #(#properties,)*
                    },
                    "required": [#(#required_fields,)*],
                    "additionalProperties": false
                })
            }
        }
    };

    expanded.into()
}

/// Generate JSON Schema for a given Rust type
fn generate_type_schema(ty: &Type) -> proc_macro2::TokenStream {
    match ty {
        Type::Path(type_path) => {
            let path = &type_path.path;
            let segment = path.segments.last().unwrap();
            let ident = &segment.ident;
            let ident_str = ident.to_string();

            match ident_str.as_str() {
                // Primitive types
                "String" | "str" => quote! { serde_json::json!({ "type": "string" }) },
                "bool" => quote! { serde_json::json!({ "type": "boolean" }) },
                "i8" | "i16" | "i32" | "i64" | "i128" | "isize" => {
                    quote! { serde_json::json!({ "type": "integer" }) }
                }
                "u8" | "u16" | "u32" | "u64" | "u128" | "usize" => {
                    quote! { serde_json::json!({ "type": "integer", "minimum": 0 }) }
                }
                "f32" | "f64" => quote! { serde_json::json!({ "type": "number" }) },

                // Handle Option<T>
                "Option" => {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                            // For Option<T>, we just return the schema for T
                            // The field won't be in the required array
                            return generate_type_schema(inner_ty);
                        }
                    }
                    quote! { serde_json::json!({ "type": ["null", "string"] }) }
                }

                // Handle Vec<T>
                "Vec" => {
                    if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                        if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                            let inner_schema = generate_type_schema(inner_ty);
                            return quote! {
                                serde_json::json!({
                                    "type": "array",
                                    "items": #inner_schema
                                })
                            };
                        }
                    }
                    quote! { serde_json::json!({ "type": "array" }) }
                }

                // Handle HashMap<K, V>
                "HashMap" | "BTreeMap" => {
                    quote! { serde_json::json!({ "type": "object" }) }
                }

                // Default to object for custom types
                _ => quote! { serde_json::json!({ "type": "object" }) },
            }
        }
        _ => quote! { serde_json::json!({ "type": "object" }) },
    }
}

/// Check if a type is Option<T>
fn is_option_type(ty: &Type) -> bool {
    if let Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            return segment.ident == "Option";
        }
    }
    false
}
