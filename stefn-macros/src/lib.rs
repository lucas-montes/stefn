use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, LitStr};

#[proc_macro_derive(ToForm, attributes(html))]
pub fn to_html_form_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);

    let struct_name = input.ident;
    let fields = match input.data {
        Data::Struct(data) => data.fields,
        _ => panic!("#[derive(ToForm)] can only be used on structs"),
    };

    let full_fields: Vec<_> = fields
        .iter()
        .map(|field| process_field(&field, true))
        .collect();

    let empty_fields: Vec<_> = fields
        .iter()
        .map(|field| process_field(&field, false))
        .collect();

    // Generate the implementation
    let expanded = quote! {
        impl stefn::ToForm for #struct_name {
            fn to_form<'a>(&self) -> stefn::HtmlTag<'a> {
                stefn::HtmlTag::Form(stefn::FormTag::new(vec![#(#full_fields),*]))
            }

            fn to_empty_form<'a>() -> stefn::HtmlTag<'a> {
                stefn::HtmlTag::Form(stefn::FormTag::new(vec![#(#empty_fields),*]))
            }
        }
    };

    TokenStream::from(expanded)
}

fn process_field(field: &syn::Field, include_value: bool) -> proc_macro2::TokenStream {
    let field_name = field.ident.as_ref().unwrap();

    field
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("html"))
        .and_then(|attr| Some(FormFieldAttributes::new(attr)))
        .unwrap_or_default()
        .to_input_tag(&field_name, include_value)
}

fn get_default_stream(tag_attribute: &Option<LitStr>) -> proc_macro2::TokenStream {
    tag_attribute.as_ref().map_or(
        quote! { std::borrow::Cow::default() },
        |c| quote! { #c.into() },
    )
}

#[derive(Default)]
struct FormFieldAttributes {
    id: Option<syn::LitStr>,
    style: Option<syn::LitStr>,
    class: Option<syn::LitStr>,
    type_: Option<syn::LitStr>,
    name: Option<syn::LitStr>,
    placeholder: Option<syn::LitStr>,
}

impl FormFieldAttributes {
    fn new(attr: &syn::Attribute) -> Self {
        let mut attrs = FormFieldAttributes::default();

        attr.parse_nested_meta(|meta| {
            if meta.path.is_ident("id") {
                attrs.id = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("style") {
                attrs.style = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("class") {
                attrs.class = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("type") {
                attrs.type_ = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("name") {
                attrs.name = Some(meta.value()?.parse()?);
            } else if meta.path.is_ident("placeholder") {
                attrs.placeholder = Some(meta.value()?.parse()?);
            }
            Ok(())
        })
        .unwrap();

        attrs
    }
    fn resolve_type(&self) -> proc_macro2::TokenStream {
        self.type_.as_ref().map_or_else(
            || quote! { stefn::InputType::Text },
            |t| {
                let type_str = t.value();
                quote! { stefn::InputType::from(#type_str) }
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

    fn to_input_tag(
        &self,
        field_name: &syn::Ident,
        include_value: bool,
    ) -> proc_macro2::TokenStream {
        let id = get_default_stream(&self.id);
        let style = get_default_stream(&self.style);
        let type_ = self.resolve_type();
        let name = self.resolve_name(field_name);
        let placeholder = get_default_stream(&self.placeholder);
        let value = self.resolve_value(field_name, include_value);
        let class = get_default_stream(&self.class);

        quote! {
            stefn::HtmlTag::Input(stefn::InputTag {
                attributes: stefn::BasicAttributes::new(#id, #class, #style),
                name: #name,
                type_: #type_,
                value: #value,
                placeholder: #placeholder,
                error: None,
                required: false,
            })
        }
    }
}
