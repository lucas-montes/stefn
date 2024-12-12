use std::borrow::Cow;

use crate::config::WebsiteConfig;

#[derive(Debug)]
pub struct Meta<'a> {
    pub meta_title: Cow<'a, str>,
    pub meta_description: Cow<'a, str>,
    pub meta_keywords: Cow<'a, str>,
    pub meta_author: Cow<'a, str>,
    pub meta_url: Cow<'a, str>,
    pub twitter: TwitterMetadata<'a>,
}

impl<'a> Default for Meta<'a> {
    fn default() -> Self {
        let package_name = env!("CARGO_PKG_NAME");
        let authors = env!("CARGO_PKG_AUTHORS");
        let description = env!("CARGO_PKG_DESCRIPTION");
        let repository = env!("CARGO_PKG_REPOSITORY");

        Self {
            meta_title: package_name.into(),
            meta_description: description.into(),
            meta_keywords: package_name.into(),
            meta_author: authors.into(),
            meta_url: repository.into(),
            twitter: TwitterMetadata::default(),
        }
    }
}

#[derive(Debug, Default)]
pub struct TwitterMetadata<'a> {
    pub site: Cow<'a, str>,
    pub title: Cow<'a, str>,
    pub description: Cow<'a, str>,
    pub creator: Cow<'a, str>,
    pub image: Cow<'a, str>,
}
