use stefn::ToForm;

#[test]
fn test_to_form_with_all_attributes() {
    //TODO: some are missing, will add them later. Add them in a nested field ex: div(class="",...), input(...)
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
        "<form id=\"form-id\" class=\"form-class\" style=\"\" method=\"POST\" action=\"\"><div id=\"usernameField-div\" class=\"form-group\" style=\"\"><label id=\"usernameField-label\" class=\"form-label\" style=\"\">user_email</label><input id=\"usernameField-input\" class=\"form-control\" style=\"\" name=\"user_email\" type_=\"email\" value=\"test@example.com\" placeholder=\"Enter your email\" /></div><button id=\"\" class=\"btn btn-primary\" style=\"\" type_=\"submit\">Save</button></form>"
    );

    assert_eq!(
        &UserCreate::to_empty_form().to_string(),
        "<form id=\"form-id\" class=\"form-class\" style=\"\" method=\"POST\" action=\"\"><div id=\"usernameField-div\" class=\"form-group\" style=\"\"><label id=\"usernameField-label\" class=\"form-label\" style=\"\">user_email</label><input id=\"usernameField-input\" class=\"form-control\" style=\"\" name=\"user_email\" type_=\"email\" value=\"\" placeholder=\"Enter your email\" /></div><button id=\"\" class=\"btn btn-primary\" style=\"\" type_=\"submit\">Save</button></form>"
    );
}
