use std::borrow::Cow;

pub struct Meta<'a> {
    pub meta_title: Cow<'a, str>,
    pub meta_description: Cow<'a, str>,
    pub meta_keywords: Cow<'a, str>,
    pub meta_author: Cow<'a, str>,
    pub meta_url: Cow<'a, str>,
    pub twitter: TwitterMetadata<'a>,
}

impl<'a> Meta<'a> {
    pub fn new(
        meta_title: Cow<'a, str>,
        meta_description: Cow<'a, str>,
        meta_keywords: Cow<'a, str>,
        meta_author: Cow<'a, str>,
        meta_url: Cow<'a, str>,
        meta_image: Cow<'a, str>,
    ) -> Meta<'a> {
        let twitter = TwitterMetadata::new(
            "elerem.com".into(),
            meta_title.clone(),
            meta_description.clone(),
            meta_author.clone(),
            meta_image,
        );
        Self {
            meta_title: meta_title,
            meta_description: meta_description.clone(),
            meta_keywords: meta_keywords.clone(),
            meta_author: meta_author.clone(),
            meta_url: meta_url.into(),
            twitter,
        }
    }
}

pub struct TwitterMetadata<'a> {
    pub site: Cow<'a, str>,
    pub title: Cow<'a, str>,
    pub description: Cow<'a, str>,
    pub creator: Cow<'a, str>,
    pub image: Cow<'a, str>,
}

impl<'a> TwitterMetadata<'a> {
    pub fn new(
        site: Cow<'a, str>,
        title: Cow<'a, str>,
        description: Cow<'a, str>,
        creator: Cow<'a, str>,
        image: Cow<'a, str>,
    ) -> Self {
        TwitterMetadata {
            site: site.into(),
            title: title,
            description: description.into(),
            creator: creator.into(),
            image: image.into(),
        }
    }
}
