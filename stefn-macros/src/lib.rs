use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, FieldsNamed, LitStr};

#[proc_macro_derive(Crud)]
pub fn add_crud_methods(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let named_fields = get_named_fields(&input);
    let from_env_with_prefix_fn = generate_from_env_with_prefix(named_fields);

    let struct_name = input.ident;

    let expanded = quote! {
        impl #struct_name {
            #from_env_with_prefix_fn

        }
    };

    TokenStream::from(expanded)
}

fn get_named_fields(input: &DeriveInput) -> &FieldsNamed {
    if let Data::Struct(data) = &input.data {
        if let Fields::Named(named_fields) = &data.fields {
            named_fields
        } else {
            panic!("FromEnvWithPrefix can only be derived for structs with named fields");
        }
    } else {
        panic!("FromEnvWithPrefix can only be derived for structs");
    }
}

fn generate_from_env_with_prefix(named_fields: &FieldsNamed) -> proc_macro2::TokenStream {
    let fields = named_fields
        .named
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string().to_uppercase();

            quote! {
                #field_name: {
                    let env_var_name = format!("{}{}", prefix, #field_name_str);
                    std::env::var(&env_var_name)
                        .expect(&format!("Environment variable `{}` not set", env_var_name))
                        .parse()
                        .expect(&format!("Failed to parse `{}`", env_var_name))
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        pub fn from_env_with_prefix(prefix: &str) -> Self {
            Self {
                #(#fields),*
            }
        }
    }
}
