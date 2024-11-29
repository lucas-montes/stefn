use std::{borrow::Cow, fmt};

#[derive(Default)]
pub struct InputTag<'a> {
    attributes: BasicAttributes<'a>,
    name: Cow<'a, str>,
    type_: InputType,
    value: Option<String>,
    placeholder: Cow<'a, str>,
    error: Option<Cow<'a, str>>,
    required: bool,
}

impl<'a> InputTag<'a> {
    pub fn new(attributes: BasicAttributes<'a>,
    name: Cow<'a, str>,
    type_: InputType,
    value: Option<String>,
    placeholder: Cow<'a, str>,
    error: Option<Cow<'a, str>>,
    required: bool,) -> Self {
        Self { attributes, name, type_, value, placeholder, error, required }
    }
}

impl<'a> fmt::Display for InputTag<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<input {} name=\"{}\" type_=\"{}\" value=\"{}\" placeholder=\"{}\"/>",
            self.attributes,
            self.name,
            self.type_,
            self.value.as_ref().map_or("", |f| f),
            self.placeholder,
        )
    }
}

#[derive(Default)]
pub enum InputType {
    #[default]
    Text,
    Number,
    Email,
    Select,
    Password,
}

impl fmt::Display for InputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text => write!(f, "text"),
            Self::Number => write!(f, "number"),
            Self::Email => write!(f, "email"),
            Self::Select => write!(f, "select"),
            Self::Password => write!(f, "password"),
        }
    }
}

pub struct FormTag<'a> {
    action: Cow<'a, str>,
    method: Cow<'a, str>,
    attributes: BasicAttributes<'a>,
    children: Vec<HtmlTag<'a>>,
}

impl<'a> FormTag<'a> {
    pub fn new(children: Vec<HtmlTag<'a>>) -> Self {
        let mut form = Self::default();
        form.children = children;
        form
    }

    pub fn set_method(mut self, method: &'a str)->Self{
        self.method = Cow::Borrowed(method);
        self
    }
    fn default() -> Self {
        Self {
            action: Cow::default(),
            method: Cow::Borrowed("POST"),
            attributes: BasicAttributes {
                id: Cow::Borrowed("form-id"),
                class: Cow::Borrowed("form-class"),
                style: Cow::Borrowed(""),
            },
            children: vec![],
        }
    }
}

impl<'a> fmt::Display for FormTag<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<form {} method=\"{}\" action=\"{}\">",
            self.attributes, self.method, self.action
        )?;
        for child in &self.children {
            write!(f, "{}", child)?;
        }
        write!(f, "</form>")
    }
}

#[derive(Default)]
pub enum ChildTag {
    #[default]
    Label,
    P,
    Span,
}

impl fmt::Display for ChildTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Label => write!(f, "label"),
            Self::P => write!(f, "p"),
            Self::Span => write!(f, "span"),
        }
    }
}

#[derive(Default)]
pub struct GeneralChildTag<'a> {
    tag: ChildTag,
    attributes: BasicAttributes<'a>,
    value: String,
}

impl<'a> GeneralChildTag<'a> {
    pub fn new(tag: ChildTag,
        attributes: BasicAttributes<'a>,
        value: String,) -> Self {
        Self { tag, attributes, value }
    }
}

impl<'a> fmt::Display for GeneralChildTag<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "<{tag} {attributes}>{value}</{tag}>",
            tag = self.tag,
            attributes = self.attributes,
            value = self.value,
        )
    }
}

pub enum ParentTag {
    Button,
    Div,
}

impl fmt::Display for ParentTag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Button => write!(f, "button"),
            Self::Div => write!(f, "div"),
        }
    }
}

pub struct GeneralParentTag<'a> {
    tag: ParentTag,
    attributes: BasicAttributes<'a>,
    children: Vec<HtmlTag<'a>>,
}

impl<'a> GeneralParentTag<'a> {
    pub fn new(tag: ParentTag,
        attributes: BasicAttributes<'a>,
        children: Vec<HtmlTag<'a>>,) -> Self {
        Self { tag, attributes, children }
    }
}

impl<'a> fmt::Display for GeneralParentTag<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<{} {}>", self.tag, self.attributes,)?;
        for child in &self.children {
            write!(f, "{}", child)?;
        }
        write!(f, "</{}>", self.tag,)
    }
}

#[derive(Default)]
pub struct BasicAttributes<'a> {
    id: Cow<'a, str>,
    class: Cow<'a, str>,
    style: Cow<'a, str>,
}

impl<'a> BasicAttributes<'a> {
    pub fn new(id: Cow<'a, str>, class: Cow<'a, str>, style: Cow<'a, str>) -> Self {
        Self { id, class, style }
    }
}

impl<'a> fmt::Display for BasicAttributes<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "id=\"{}\" class=\"{}\" style=\"{}\"",
            self.id, self.class, self.style
        )
    }
}

pub enum HtmlTag<'a> {
    Form(FormTag<'a>),
    ParentTag(GeneralParentTag<'a>),
    Input(InputTag<'a>),
    ChildTag(GeneralChildTag<'a>),
}

impl<'a> fmt::Display for HtmlTag<'a> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Form(tag) => {
                write!(f, "{}", tag)
            }
            Self::ParentTag(tag) => {
                write!(f, "{}", tag)
            }
            Self::Input(tag) => {
                write!(f, "{}", tag)
            }
            Self::ChildTag(tag) => {
                write!(f, "{}", tag)
            }
        }
    }
}

pub trait ToForm {
    fn to_form<'a>(&self) -> HtmlTag<'a>;
    fn to_empty_form<'a>() -> HtmlTag<'a>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_form_tag() {
        let result = HtmlTag::Form(FormTag::default());
        assert_eq!(&result.to_string(), "<form id=\"form-id\" class=\"form-class\" style=\"\" method=\"POST\" action=\"\"></form>");
    }

    #[test]
    fn test_form_tag_with_children() {
        let children = HtmlTag::Input(InputTag::default());
        let result = HtmlTag::Form(FormTag::new(vec![children]));
        assert_eq!(
            &result.to_string(), 
            "<form id=\"form-id\" class=\"form-class\" style=\"\" method=\"POST\" action=\"\"><input id=\"\" class=\"\" style=\"\" name=\"\" type_=\"text\" value=\"\" placeholder=\"\"/></form>"
        );
    }
}