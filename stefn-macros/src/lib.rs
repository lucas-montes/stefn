use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Data, DeriveInput, LitStr};

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
        .map(|field| process_field::<S>(&field, true))
        .collect();

    let empty_fields: Vec<_> = fields
        .iter()
        .map(|field| process_field::<S>(&field, false))
        .collect();

    let button_class = S::button_form_class();
    let button_value = S::button_form_value();

    let expanded = quote! {
        impl stefn::html::ToForm for #struct_name {
            fn form_button<'a>() -> stefn::html::HtmlTag<'a> {
                stefn::html::HtmlTag::ParentTag(stefn::html::GeneralParentTag::submit_button(#button_value.into(), #button_class.into()))
            }

            fn raw_form<'a>(&self) -> stefn::html::FormTag<'a> {
                stefn::html::FormTag::new(vec![#(#full_fields),*])
            }

            fn raw_empty_form<'a>() -> stefn::html::FormTag<'a> {
                stefn::html::FormTag::new(vec![#(#empty_fields),*])
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
        .and_then(|attr| Some(FormFieldAttributes::new(attr)))
        .unwrap_or_default()
        .to_form_input::<S>(&field_name, include_value)
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
            || quote! { stefn::html::InputType::Text },
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
        unique_id.push_str("-");
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
        let div_id = self.generate_id(&field_name, "div");
        let input_id = self.generate_id(&field_name, "input");
        let label_id = self.generate_id(&field_name, "label");

        let style = get_default_stream(&self.style);
        let type_ = self.resolve_type();
        let name = self.resolve_name(field_name);
        let placeholder = get_default_stream(&self.placeholder);
        let value = self.resolve_value(field_name, include_value);

        let div_class = self.resolve_div_class::<S>();
        let input_class = self.resolve_input_class::<S>();
        let label_class = self.resolve_label_class::<S>();

        quote! {
            stefn::html::HtmlTag::ParentTag(stefn::html::GeneralParentTag::new(
                stefn::html::ParentTag::Div,
                stefn::html::BasicAttributes::new(#div_id, #div_class, #style),
                vec![
                    stefn::html::HtmlTag::ChildTag(stefn::html::GeneralChildTag::new(
                        Some(stefn::html::ChildTag::Label),
                        stefn::html::BasicAttributes::new(#label_id, #label_class, #style),
                        #name,
                    )),
                    stefn::html::HtmlTag::Input(stefn::html::InputTag::new(
                        stefn::html::BasicAttributes::new(#input_id, #input_class, #style),
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
