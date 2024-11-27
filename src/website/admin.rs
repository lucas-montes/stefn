use std::borrow::Cow;

use askama::Template;
use axum::{
    async_trait,
    extract::{FromRef, Path, State},
    response::Redirect,
    routing::get,
    Form, Router,
};
use serde::{de::DeserializeOwned, Serialize};
use sqlx::sqlite::SqliteRow;

use crate::{
    database::{Manager, Where},
    AppError, Database, WebsiteState,
};

use super::{seo::Meta, HtmlTag, ToForm};

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate<'a> {
    meta: Meta<'a>,
}

#[derive(Template)]
#[template(path = "admin/form.html")]
pub struct AdminFormTemplate<'a> {
    meta: Meta<'a>,
    form: HtmlTag<'a>,
}

#[async_trait]
pub trait Admin: Manager
where
    Self: Send + 'static,
{
    const PATH: &str = "";
    type Create: Send + DeserializeOwned + Serialize + Default + ToForm;
    type Update: Send + DeserializeOwned + ToForm + Unpin + for<'r> sqlx::FromRow<'r, SqliteRow>;
    type Read: Send;

    fn entity_name() -> String {
        std::any::type_name::<Self>()
            .rsplit("::")
            .next()
            .unwrap()
            .to_lowercase()
    }

    fn base_path<'a>() -> Cow<'a, str> {
        if Self::PATH.is_empty() {
            return format!("/{}s", Self::entity_name()).into();
        }
        Cow::Borrowed(Self::PATH)
    }

    fn routes(state: WebsiteState) -> Router<WebsiteState> {
        //TODO: instead of the webiste state use a generic with the fromRef with the databse to have access to it
        let base_path = Self::base_path();
        //TODO: avoid to much string
        Router::new()
            .route(&base_path, get(Self::list))
            .route(
                &format!("{}/new", base_path),
                get(Self::get_create_form).post(Self::post),
            )
            .route(
                &format!("{}/:id", base_path),
                get(Self::get).post(Self::patch).delete(Self::delete),
                //TODO: change post to patch and set the method to the form
            )
            .with_state(state)
    }

    async fn list<'a>() -> IndexTemplate<'a> {
        let meta = Meta::new(
            "dashboard title".into(),
            "elerem mola".into(),
            "recsys,mola".into(),
            "lucas montes".into(),
            "elerem.com".into(),
            "imafge.com".into(),
        );
        IndexTemplate { meta }
    }

    async fn post(Form(payload): Form<Self::Create>) -> Result<Redirect, AppError> {
        Ok(Self::post_redirect(2))
    }

    fn post_redirect(model_pk: i64) -> Redirect {
        Redirect::to(&format!("{}{}/", Self::base_path(), model_pk))
    }

    async fn get_create_form<'a>() -> AdminFormTemplate<'a> {
        let meta = Meta::new(
            "dashboard title".into(),
            "elerem mola".into(),
            "recsys,mola".into(),
            "lucas montes".into(),
            "elerem.com".into(),
            "imafge.com".into(),
        );
        let form = Self::Create::to_empty_form();
        AdminFormTemplate { meta, form }
    }

    async fn get<'a>(
        State(database): State<Database>,
        Path(model_pk): Path<i64>,
    ) -> Result<AdminFormTemplate<'a>, AppError> {
        let meta = Meta::new(
            "dashboard title".into(),
            "elerem mola".into(),
            "recsys,mola".into(),
            "lucas montes".into(),
            "elerem.com".into(),
            "imafge.com".into(),
        );
        let model = Self::get_by::<Self::Update>(&database, Where::Pk(model_pk))
            .await?
            .ok_or_else(|| AppError::DoesNotExist)?;

        let form = model.to_form();
        Ok(AdminFormTemplate { meta, form })
    }

    async fn patch(
        State(database): State<Database>,
        Path(model_pk): Path<i64>,
        Form(payload): Form<Self::Update>,
    ) -> Result<Redirect, AppError> {
        Ok(Self::patch_redirect(model_pk))
    }

    fn patch_redirect(model_pk: i64) -> Redirect {
        Redirect::to(&format!("{}{}/", Self::base_path(), model_pk))
    }

    async fn delete(
        State(database): State<Database>,
        Path(model_pk): Path<i64>,
    ) -> Result<Redirect, AppError> {
        Self::delete_by(&database, Where::from_pk(model_pk)).await?;
        Ok(Self::delete_redirect())
    }

    fn delete_redirect() -> Redirect {
        Redirect::to(&Self::base_path())
    }
}
