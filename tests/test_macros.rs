use stefn::{forms::ToForm, ToForm};
use stefn_macros::ToBootstrapForm;

#[test]
fn test_to_form_with_all_attributes() {
    //TODO: some are missing, will add them later (LOL). Add them in a nested field ex: div(class="",...), input(...)
    #[derive(ToForm)]
    struct UserCreate {
        #[html(
            id = "usernameField",
            div_class = "form-group",
            input_class = "form-control",
            label_class = "form-label",
            type_ = "email",
            name = "user_email",
            label = "Email",
            placeholder = "Enter your email"
        )]
        email: String,
    }

    let user = UserCreate {
        email: "test@example.com".into(),
    };

    assert_eq!(
        &user.to_form().to_string(),
        "<form id=\"form-id\" class=\"form-class\" style=\"\" method=\"POST\" action=\"\"><div id=\"usernameField-div\" class=\"form-group\" style=\"\"><label id=\"usernameField-label\" class=\"form-label\" style=\"\">user_email</label><input id=\"usernameField-input\" class=\"form-control\" style=\"\" name=\"user_email\" type_=\"email\" value=\"test@example.com\" placeholder=\"Enter your email\"/></div></form>"
    );

    assert_eq!(
        &UserCreate::to_empty_form().to_string(),
        "<form id=\"form-id\" class=\"form-class\" style=\"\" method=\"POST\" action=\"\"><div id=\"usernameField-div\" class=\"form-group\" style=\"\"><label id=\"usernameField-label\" class=\"form-label\" style=\"\">user_email</label><input id=\"usernameField-input\" class=\"form-control\" style=\"\" name=\"user_email\" type_=\"email\" value=\"\" placeholder=\"Enter your email\"/></div></form>"
    );
}

#[test]
fn test_to_bootstrap_form_with_custom_classes() {
    #[derive(ToBootstrapForm)]
    struct UserCreate {
        #[html(id = "passwordField", label = "Password")]
        password: String,
    }

    let user = UserCreate {
        password: "secret".into(),
    };

    assert_eq!(
        &user.to_form().to_string(),
        "<form id=\"form-id\" class=\"form-class\" style=\"\" method=\"POST\" action=\"\"><div id=\"passwordField-div\" class=\"mb-3\" style=\"\"><label id=\"passwordField-label\" class=\"form-label\" style=\"\">password</label><input id=\"passwordField-input\" class=\"form-control\" style=\"\" name=\"password\" type_=\"text\" value=\"secret\" placeholder=\"\"/></div></form>"
    );

    assert_eq!(
        &UserCreate::to_empty_form().to_string(),
        "<form id=\"form-id\" class=\"form-class\" style=\"\" method=\"POST\" action=\"\"><div id=\"passwordField-div\" class=\"mb-3\" style=\"\"><label id=\"passwordField-label\" class=\"form-label\" style=\"\">password</label><input id=\"passwordField-input\" class=\"form-control\" style=\"\" name=\"password\" type_=\"text\" value=\"\" placeholder=\"\"/></div></form>"
    );
}
