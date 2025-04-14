use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, DeriveInput};

#[proc_macro_derive(FromEnv)]
pub fn from_env_macro(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name = &input.ident;

    // Extract struct fields
    let fields = if let syn::Data::Struct(s) = input.data {
        s.fields
    } else {
        panic!("FromEnv can only be used on structs");
    };

    // Generate parsing logic for each field
    let field_parsing = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let field_type = &f.ty;
        let env_var = field_name.to_string().to_uppercase(); // Convert to ENV_VAR format

        quote! {
            #field_name: std::env::var(#env_var)
                .expect(&format!("Missing env var: {}", #env_var))
                .parse::<#field_type>()
                .expect(&format!("Failed to parse env var: {}", #env_var))
        }
    });

    // Generate print statements for each field
    let field_print = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let field_str = field_name.to_string();

        quote! {
            log::info!("  {}: {}", #field_str, self.#field_name);
        }
    });

    // Generate parsing logic for each field from file
    let field_parsing_file = fields.iter().map(|f| {
        let field_name = f.ident.as_ref().unwrap();
        let field_type = &f.ty;
        let env_var = field_name.to_string().to_uppercase(); // Convert to ENV_VAR format

        quote! {
            #field_name: config.get(#env_var)
                .expect(&format!("Missing config key: {}", #env_var))
                .parse::<#field_type>()
                .expect(&format!("Failed to parse config value: {}", #env_var))
        }
    });

    let expanded = quote! {
        impl #struct_name {
            pub fn from_env() -> Self {
                let config = Self {
                    #(#field_parsing),*
                };
                log::info!("Configuration loaded from environment:");
                config.print();
                config
            }

            pub fn from_file(path: &str) -> Self {
                use std::collections::HashMap;
                use std::fs::File;
                use std::io::{BufRead, BufReader};

                let file = File::open(path).expect("Failed to open config file");
                let reader = BufReader::new(file);
                let mut config: HashMap<String, String> = HashMap::new();

                for line in reader.lines() {
                    let line = line.expect("Failed to read line from config file");
                    if line.trim().starts_with('#') || line.trim().is_empty() {
                        continue;
                    }

                    if let Some((key, value)) = line.split_once('=') {
                        config.insert(key.trim().to_uppercase(), value.trim().to_string());
                    }
                }

                let config = Self {
                    #(#field_parsing_file),*
                };
                log::info!("Configuration loaded from file '{}':", path);
                config.print();
                config
            }

            fn print(&self) {
                #(#field_print)*
            }
        }
    };

    TokenStream::from(expanded)
}
