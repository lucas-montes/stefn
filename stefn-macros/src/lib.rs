use proc_macro::TokenStream;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Data, DeriveInput, LitStr};

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
        impl stefn::forms::ToForm for #struct_name {
            fn to_form<'a>(&self) -> stefn::forms::HtmlTag<'a> {
                stefn::forms::HtmlTag::Form(stefn::forms::FormTag::new(vec![#(#full_fields),*]))
            }

            fn to_empty_form<'a>() -> stefn::forms::HtmlTag<'a> {
                stefn::forms::HtmlTag::Form(stefn::forms::FormTag::new(vec![#(#empty_fields),*]))
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
                Some("type") => attrs.type_ = Some(value),
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
            || quote! { stefn::forms::InputType::Text },
            |t| {
                let type_str = t.value();
                quote! { stefn::forms::InputType::from(#type_str) }
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

    fn to_input_tag(
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

        let div_class = get_default_stream(&self.div_class);
        let input_class = get_default_stream(&self.input_class);
        let label_class = get_default_stream(&self.label_class);

        quote! {
            stefn::forms::HtmlTag::ParentTag(stefn::forms::GeneralParentTag::new(
                stefn::forms::ParentTag::Div,
                stefn::forms::BasicAttributes::new(#div_id, #div_class, #style),
                vec![
                    stefn::forms::HtmlTag::ChildTag(stefn::forms::GeneralChildTag::new(
                        stefn::forms::ChildTag::Label,
                        stefn::forms::BasicAttributes::new(#label_id, #label_class, #style),
                        #name,
                    )),
                    stefn::forms::HtmlTag::Input(stefn::forms::InputTag::new(
                        stefn::forms::BasicAttributes::new(#input_id, #input_class, #style),
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
