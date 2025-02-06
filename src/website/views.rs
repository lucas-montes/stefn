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
        struct Error404<'a> {
            meta: ::stefn::website::Meta<'a>,
        }

        #[derive(askama::Template)]
        #[template(path = $internal_error_template)]
        struct Error500<'a> {
            meta: ::stefn::website::Meta<'a>,
        }

        pub struct HtmlError(axum::http::StatusCode, String);

        impl HtmlError {
            fn not_found<'a>() -> Error404<'a> {
                let meta = ::stefn::website::Meta {
                    meta_title: "Not Found".into(),
                    ..Default::default()
                };

                Error404 { meta }
            }

            fn internal_error<'a>() -> Error500<'a> {
                let meta = ::stefn::website::Meta {
                    meta_title: "Server Error".into(),
                    ..Default::default()
                };

                Error500 { meta }
            }
        }

        impl ::axum::response::IntoResponse for HtmlError {
            fn into_response(self) -> ::axum::response::Response {
                match self.0 {
                    ::axum::http::StatusCode::NOT_FOUND => HtmlError::not_found().into_response(),
                    _ => HtmlError::internal_error().into_response(),
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
