use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use syn::{parse_macro_input, spanned::Spanned, Data, DeriveInput, Fields, LitStr, Meta};

#[proc_macro_derive(Insertable, attributes(table_name))]
pub fn add_insertable(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = input.ident;
    let table_name = input
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident("table_name") {
                if let Meta::NameValue(v) = &attr.meta {
                    return Some(v.value.to_token_stream().to_string());
                }
            }
            None
        })
        .expect("Missing attribute: table_name");

    let fields = if let Data::Struct(data_struct) = &input.data {
        match &data_struct.fields {
            Fields::Named(fields_named) => &fields_named.named,
            _ => panic!("Insertable can only be used with named structs"),
        }
    } else {
        panic!("Insertable can only be used with structs");
    };

    let field_names: Vec<_> = fields
        .iter()
        .map(|field| field.ident.as_ref().unwrap())
        .collect();
    let field_names_str = field_names
        .iter()
        .map(|ident| ident.to_string())
        .collect::<Vec<String>>()
        .join(",");
    let dolars = (1..=field_names.len())
        .map(|s| format!("${}", s))
        .collect::<Vec<String>>()
        .join(",");

    let expanded = quote! {
    impl #struct_name {
        pub fn insert_query()->String{
            format!(
                "INSERT INTO {} ({}) VALUES ({})",
                #table_name,
                #field_names_str,
                #dolars
            )
        }

        pub async fn save<'e, E>(&self, executor: E) -> Result<i64, stefn::service::AppError>
        where
            E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
        {
            sqlx::query(&Self::insert_query())
                #(.bind(&self.#field_names))*
                .execute(executor)
                .await
                .map_err(stefn::service::AppError::from)
                .map(|q| q.last_insert_rowid())
        }
            }
        };

    TokenStream::from(expanded)
}

#[proc_macro_derive(CsrfProtected)]
pub fn add_csrf_token_derive(input: TokenStream) -> TokenStream {
    //TODO: make it work
    let input = parse_macro_input!(input as DeriveInput);

    // Extract the struct's identifier (name)
    let struct_name = input.ident;

    // Extract fields from the struct
    let fields = if let Data::Struct(data_struct) = &input.data {
        match &data_struct.fields {
            Fields::Named(fields_named) => &fields_named.named,
            _ => panic!("AddCsrfToken can only be used with named structs"),
        }
    } else {
        panic!("AddCsrfToken can only be used with structs");
    };

    // Generate the expanded fields: existing fields + csrf_token
    let mut expanded_fields = quote! {};
    for field in fields {
        let field_name = &field.ident;
        let field_type = &field.ty;
        expanded_fields = quote! {
            #expanded_fields
            #field_name: #field_type,
        };
    }

    // Add the csrf_token field
    expanded_fields = quote! {
        #expanded_fields
        pub csrf_token: String,
    };

    // Generate the implementation
    let expanded = quote! {
        #[derive(Debug)]
        pub struct #struct_name {
            #expanded_fields
        }

        // impl #struct_name {
        //     pub fn new_with_csrf(csrf_token: String) -> Self {
        //         Self {
        //             csrf_token,
        //             ..Default::default()
        //         }
        //     }
        // }
    };

    // Return the generated code
    TokenStream::from(expanded)
}

#[proc_macro_derive(ToForm, attributes(html))]
pub fn to_regular_form(input: TokenStream) -> TokenStream {
    to_html_form_derive::<BootstrapStyle>(input)
}

fn to_html_form_derive<S: FormStyle>(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = input.ident;
    let fields = match input.data {
        Data::Struct(data) => data.fields,
        _ => panic!("#[derive(ToForm)] can only be used on structs"),
    };

    let full_fields: Vec<_> = fields
        .iter()
        .map(|field| process_field::<S>(field, true))
        .collect();

    let empty_fields: Vec<_> = fields
        .iter()
        .map(|field| process_field::<S>(field, false))
        .collect();

    let button_class = S::button_form_class();
    let button_value = S::button_form_value();

    let expanded = quote! {
        impl stefn::website::html::ToForm for #struct_name {
            fn form_button<'a>() -> stefn::website::html::HtmlTag<'a> {
                stefn::website::html::HtmlTag::ParentTag(stefn::website::html::GeneralParentTag::submit_button(#button_value.into(), #button_class.into()))
            }

            fn raw_form<'a>(&self) -> stefn::website::html::FormTag<'a> {
                stefn::website::html::FormTag::new(vec![#(#full_fields),*])
            }

            fn raw_empty_form<'a>() -> stefn::website::html::FormTag<'a> {
                stefn::website::html::FormTag::new(vec![#(#empty_fields),*])
            }
        }
    };

    TokenStream::from(expanded)
}

fn process_field<S: FormStyle>(
    field: &syn::Field,
    include_value: bool,
) -> proc_macro2::TokenStream {
    let field_name = field.ident.as_ref().unwrap();

    field
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("html"))
        .map(FormFieldAttributes::new)
        .unwrap_or_default()
        .to_form_input::<S>(field_name, include_value)
}

fn get_default_stream(tag_attribute: &Option<LitStr>) -> proc_macro2::TokenStream {
    tag_attribute.as_ref().map_or(
        quote! { std::borrow::Cow::default() },
        |c| quote! { #c.into() },
    )
}

trait FormStyle {
    fn button_form_class() -> &'static str {
        "btn btn-primary"
    }
    fn button_form_value() -> &'static str {
        "Save"
    }
    fn div_class() -> &'static str {
        "mb-3"
    }

    fn input_class() -> &'static str {
        "form-control"
    }

    fn label_class() -> &'static str {
        "form-label"
    }
}

struct BootstrapStyle;

impl FormStyle for BootstrapStyle {}

#[derive(Default)]
struct FormFieldAttributes {
    id: Option<LitStr>,
    style: Option<LitStr>,
    div_class: Option<LitStr>,
    input_class: Option<LitStr>,
    label_class: Option<LitStr>,
    type_: Option<LitStr>,
    name: Option<LitStr>,
    label: Option<LitStr>,
    placeholder: Option<LitStr>,
}

impl FormFieldAttributes {
    fn new(attr: &syn::Attribute) -> Self {
        let mut attrs = FormFieldAttributes::default();

        attr.parse_nested_meta(|meta| {
            let key = meta.path.get_ident().map(|id| id.to_string());
            let value: LitStr = meta.value()?.parse()?;

            match key.as_deref() {
                Some("id") => attrs.id = Some(value),
                Some("style") => attrs.style = Some(value),
                Some("div_class") => attrs.div_class = Some(value),
                Some("input_class") => attrs.input_class = Some(value),
                Some("label_class") => attrs.label_class = Some(value),
                Some("type_") => attrs.type_ = Some(value),
                Some("name") => attrs.name = Some(value),
                Some("placeholder") => attrs.placeholder = Some(value),
                Some("label") => attrs.label = Some(value),
                Some(v) => {
                    return Err(syn::Error::new(
                        meta.path.span(),
                        format!("Unknown attribute {}", v),
                    ))
                }
                None => {}
            }
            Ok(())
        })
        .unwrap_or_else(|err| panic!("Error parsing attributes : {}", err));

        attrs
    }

    fn resolve_type(&self) -> proc_macro2::TokenStream {
        self.type_.as_ref().map_or_else(
            || quote! { stefn::website::html::InputType::Text },
            |t| {
                let type_str = t.value();
                quote! { std::str::FromStr::from_str(#type_str).unwrap() }
            },
        )
    }

    fn resolve_name(&self, field_name: &syn::Ident) -> proc_macro2::TokenStream {
        self.name.as_ref().map_or_else(
            || quote! { stringify!(#field_name).into() },
            |n| quote! { #n.into() },
        )
    }

    fn resolve_value(
        &self,
        field_name: &syn::Ident,
        include_value: bool,
    ) -> proc_macro2::TokenStream {
        if include_value {
            quote! { Some(self.#field_name.to_string()) }
        } else {
            quote! { None }
        }
    }

    fn generate_id(&self, field_name: &syn::Ident, tag: &str) -> proc_macro2::TokenStream {
        let mut unique_id = self
            .id
            .as_ref()
            .map_or(field_name.to_string(), |i| i.value());
        unique_id.push('-');
        unique_id.push_str(tag);
        quote! { #unique_id.into() }
    }

    fn resolve_div_class<S: FormStyle>(&self) -> proc_macro2::TokenStream {
        let default_value = S::div_class();
        self.div_class.as_ref().map_or_else(
            || quote! { #default_value.into() },
            |class| quote! { #class.into() },
        )
    }

    fn resolve_input_class<S: FormStyle>(&self) -> proc_macro2::TokenStream {
        let default_value = S::input_class();
        self.input_class.as_ref().map_or_else(
            || quote! { #default_value.into() },
            |class| quote! { #class.into() },
        )
    }

    fn resolve_label_class<S: FormStyle>(&self) -> proc_macro2::TokenStream {
        let default_value = S::label_class();
        self.label_class.as_ref().map_or_else(
            || quote! { #default_value.into() },
            |class| quote! { #class.into() },
        )
    }

    fn to_form_input<S: FormStyle>(
        &self,
        field_name: &syn::Ident,
        include_value: bool,
    ) -> proc_macro2::TokenStream {
        let div_id = self.generate_id(field_name, "div");
        let input_id = self.generate_id(field_name, "input");
        let label_id = self.generate_id(field_name, "label");

        let style = get_default_stream(&self.style);
        let type_ = self.resolve_type();
        let name = self.resolve_name(field_name);
        let placeholder = get_default_stream(&self.placeholder);
        let value = self.resolve_value(field_name, include_value);

        let div_class = self.resolve_div_class::<S>();
        let input_class = self.resolve_input_class::<S>();
        let label_class = self.resolve_label_class::<S>();

        quote! {
            stefn::website::html::HtmlTag::ParentTag(stefn::website::html::GeneralParentTag::new(
                stefn::website::html::ParentTag::Div,
                stefn::website::html::BasicAttributes::new(#div_id, #div_class, #style),
                vec![
                    stefn::website::html::HtmlTag::ChildTag(stefn::website::html::GeneralChildTag::new(
                        Some(stefn::website::html::ChildTag::Label),
                        stefn::website::html::BasicAttributes::new(#label_id, #label_class, #style),
                        #name,
                    )),
                    stefn::website::html::HtmlTag::Input(stefn::website::html::InputTag::new(
                        stefn::website::html::BasicAttributes::new(#input_id, #input_class, #style),
                        #name,
                        #type_,
                        #value,
                        #placeholder,
                        None,
                        false,
                    ))
                ],
            ))

        }
    }
}
