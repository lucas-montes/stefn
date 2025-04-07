use std::borrow::Cow;

#[derive(Debug)]
pub struct Meta<'a> {
    pub meta_title: Cow<'a, str>,
    pub meta_description: Cow<'a, str>,
    pub meta_keywords: Cow<'a, str>,
    pub meta_author: Cow<'a, str>,
    pub meta_url: Cow<'a, str>,
    pub csp_policy: ContentSecurityPolicy<'a>,
    pub twitter: TwitterMetadata<'a>,
}

impl Default for Meta<'_> {
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
csp_policy: ContentSecurityPolicy::default(),            twitter: TwitterMetadata::default(),
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


#[derive(Debug)]
pub struct ContentSecurityPolicy<'a> {
    pub base_uri: Cow<'a, str>,
    pub child_src: Cow<'a, str>,
    pub connect_src: Cow<'a, str>,
    pub default_src: Cow<'a, str>,
    pub font_src: Cow<'a, str>,
    pub form_action: Cow<'a, str>,
    pub frame_ancestors: Cow<'a, str>,
    pub frame_src: Cow<'a, str>,
    pub img_src: Cow<'a, str>,
    pub manifest_src: Cow<'a, str>,
    pub media_src: Cow<'a, str>,
    pub object_src: Cow<'a, str>,
    pub report_to: Cow<'a, str>,
    pub require_trusted_types_for: Cow<'a, str>,
    pub sandbox: Cow<'a, str>,
    pub script_src: Cow<'a, str>,
    pub script_src_attr: Cow<'a, str>,
    pub script_src_elem: Cow<'a, str>,
    pub style_src: Cow<'a, str>,
    pub style_src_attr: Cow<'a, str>,
    pub style_src_elem: Cow<'a, str>,
    pub trusted_types: Cow<'a, str>,
    pub worker_src: Cow<'a, str>,
}

impl Default for ContentSecurityPolicy<'_> {
    fn default() -> Self {
        Self {
            base_uri: "'self'".into(),
            child_src: "'self'".into(),
            connect_src: "'self'".into(),
            default_src: "'self'".into(),
            font_src: "'self'".into(),
            form_action: "'self'".into(),
            frame_ancestors: "'self'".into(),
            frame_src: "'self'".into(),
            img_src: "'self' ".into(),
            manifest_src: "'self'".into(),
            media_src: "'self'".into(),
            object_src: "'none'".into(),
            report_to: "default".into(),
            require_trusted_types_for: "'none'".into(),
            sandbox: "allow-forms allow-same-origin allow-scripts allow-popups allow-modals allow-downloads allow-presentation allow-top-navigation allow-storage-access-by-user-activation".into(),
            script_src: "'self'".into(),
            script_src_attr: "'self'".into(),
            script_src_elem: "'self'".into(),
            style_src: "'self'".into(),
            style_src_attr: "'self'".into(),
            style_src_elem: "'self'".into(),
            trusted_types: "default".into(),
            worker_src: "'none'".into(),
        }
    }
}

impl<'a> std::fmt::Display for ContentSecurityPolicy<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, r#"<meta http-equiv="Content-Security-Policy" content=""#)?;
        write!(f, "base-uri {}; ", self.base_uri)?;
        write!(f, "child-src {}; ", self.child_src)?;
        write!(f, "connect-src {}; ", self.connect_src)?;
        write!(f, "default-src {}; ", self.default_src)?;
        write!(f, "font-src {}; ", self.font_src)?;
        write!(f, "form-action {}; ", self.form_action)?;
        write!(f, "frame-ancestors {}; ", self.frame_ancestors)?;
        write!(f, "frame-src {}; ", self.frame_src)?;
        write!(f, "img-src {}; ", self.img_src)?;
        write!(f, "manifest-src {}; ", self.manifest_src)?;
        write!(f, "media-src {}; ", self.media_src)?;
        write!(f, "object-src {}; ", self.object_src)?;
        write!(f, "report-to {}; ", self.report_to)?;
        write!(f, "require-trusted-types-for {}; ", self.require_trusted_types_for)?;
        write!(f, "sandbox {}; ", self.sandbox)?;
        write!(f, "script-src {}; ", self.script_src)?;
        write!(f, "script-src-attr {}; ", self.script_src_attr)?;
        write!(f, "script-src-elem {}; ", self.script_src_elem)?;
        write!(f, "style-src {}; ", self.style_src)?;
        write!(f, "style-src-attr {}; ", self.style_src_attr)?;
        write!(f, "style-src-elem {}; ", self.style_src_elem)?;
        write!(f, "trusted-types {}; ", self.trusted_types)?;
        write!(f, "worker-src {}", self.worker_src)?;
        write!(f, r#"" />"#)
    }
}
