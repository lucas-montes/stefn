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
            meta: ::stefn::website::Meta<'a>,
        }

        pub async fn $fn_name<'a>() -> $name<'a> {
            let meta = ::stefn::website::Meta {
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

#[macro_export]
macro_rules! create_error_templates {
    ($not_found_template:expr, $internal_error_template:expr) => {
        #[derive(askama::Template)]
        #[template(path = $not_found_template)]
        struct Error404;

        #[derive(askama::Template)]
        #[template(path = $internal_error_template)]
        struct Error500;

        pub struct HtmlError(axum::http::StatusCode, String);

        impl ::axum::response::IntoResponse for HtmlError {
            fn into_response(self) -> ::axum::response::Response {
                match self.0 {
                    axum::http::StatusCode::NOT_FOUND => Error404 {}.into_response(),
                    _ => Error500 {}.into_response(),
                }
            }
        }

        impl From<::stefn::service::AppError> for HtmlError {
            fn from(error: ::stefn::service::AppError) -> Self {
                let (status, message) = error.get_status_code_and_message();
                HtmlError(status, message)
            }
        }
    };
}
