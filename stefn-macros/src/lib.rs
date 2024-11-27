use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Fields, FieldsNamed, Meta};

// fn create_inputs_from_attributes(named_fields: &FieldsNamed) -> proc_macro2::TokenStream {
//     let fields = named_fields
//         .named
//         .iter()
//         .map(|field| {
//             let field_name = field.ident.as_ref().unwrap();
//             let field_name_str = field_name.to_string().to_uppercase();

//             let env_var_name = get_env_var_name(field, field_name_str);

//             quote! {
//                 InputTag {
//                     attributes: BasicAttributes {
//                         id: Cow::Borrowed(""),
//                         class: Cow::Borrowed("form-control"),
//                         style: Cow::Borrowed(""),
//                     },
//                     name: Cow::Borrowed(#field_name),
//                     type_: match #field_type.as_str() {
//                         "text" => InputType::Text,
//                         "email" => InputType::Email,
//                         "number" => InputType::Number,
//                         "password" => InputType::Password,
//                         "select" => InputType::Select,
//                         _ => InputType::Text,
//                     },
//                     value: None,
//                     placeholder: #placeholder.map(Cow::Borrowed),
//                     error: None,
//                     required: #required,
//                 }
//             }
//         })
//         .collect::<Vec<_>>();

//     quote! {
//         pub fn from_env() -> Self {
//             Self {
//                 #(#fields),*
//             }
//         }
//     }
// }

// fn get_env_var_name(field: &syn::Field, field_name_str: String) -> String {
//     field
//         .attrs
//         .iter()
//         .find(|attr| attr.path().is_ident("form_field"))
//         .map(|attr| {
//             let mut prefix = attr
//                 .parse_args::<LitStr>()
//                 .expect(&format!("Prefix for `{}` has a problem", field_name_str))
//                 .value();
//             prefix.push_str(&field_name_str);
//             prefix
//         })
//         .unwrap_or(field_name_str)
// }

#[proc_macro_derive(ToForm, attributes(form_field))]
pub fn to_html_form_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let named_fields = get_named_fields(&input);
    let struct_name = &input.ident;

    // let fields = if let Data::Struct(data) = &input.data {
    //     data.fields.iter().map(|field| {
    //         let field_name = field.ident.as_ref().unwrap().to_string();
    //         let mut field_type = "text".to_string();
    //         let mut placeholder = None;
    //         let mut required = false;

    //         quote! {
    //             InputTag {
    //                 attributes: BasicAttributes {
    //                     id: Cow::Borrowed(""),
    //                     class: Cow::Borrowed("form-control"),
    //                     style: Cow::Borrowed(""),
    //                 },
    //                 name: Cow::Borrowed(#field_name),
    //                 type_: match #field_type.as_str() {
    //                     "text" => InputType::Text,
    //                     "email" => InputType::Email,
    //                     "number" => InputType::Number,
    //                     "password" => InputType::Password,
    //                     "select" => InputType::Select,
    //                     _ => InputType::Text,
    //                 },
    //                 value: None,
    //                 placeholder: #placeholder.map(Cow::Borrowed),
    //                 error: None,
    //                 required: #required,
    //             }
    //         }
    //     })
    // } else {
    //     panic!("ToHtmlForm can only be derived for structs");
    // };

    let expanded = quote! {
        impl stefn::ToForm for #struct_name {
            fn to_form<'a>(&self) -> stefn::HtmlTag<'a> {
                stefn::HtmlTag::Form(stefn::FormTag::new(vec![]))
            }

            fn to_empty_form<'a>() -> stefn::HtmlTag<'a>
            {
                stefn::HtmlTag::Form(stefn::FormTag::new(vec![]))
            }
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
