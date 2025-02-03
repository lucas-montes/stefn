
#[macro_export]
macro_rules! create_view {
    ($name:ident,$fn_name:ident, $template_path:expr, {
        meta_title: $title:expr,
        meta_description: $description:expr,
        meta_keywords: $keywords:expr,
        meta_author: $author:expr,
        meta_url: $url:expr,
    }) => {
        #[derive(Template)]
        #[template(path =  $template_path)]
        pub struct $name<'a> {
            meta: ::stefn::website::seo::Meta<'a>,
        }

        pub async fn $fn_name<'a>() -> $name<'a> {
            let meta = ::stefn::website::seo::Meta {
                meta_title: $title.into(),
                meta_description: $description.into(),
                meta_keywords: $keywords.into(),
                meta_author: $author.into(),
                meta_url: $url.into(),
                ..Default::default()
            };
            $name { meta }
        }
    };
}
