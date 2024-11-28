use stefn::ToForm;

#[test]
fn test_macro() {
    #[derive(ToForm)]
    struct UserCreate {
        #[html(id = "EarlGrey", class = "primary")]
        username: String,
    }

    let user = UserCreate {
        username: "hey".into(),
    };

    assert_eq!(
        &user.to_form().to_string(),
        "<form id=\"form-id\" class=\"form-class\" style=\"\" method=\"POST\" action=\"\"><input id=\"EarlGrey\" class=\"primary\" style=\"\" name=\"username\" type_=\"text\" value=\"hey\" placeholder=\"\"/></form>"
    );

    assert_eq!(
        &UserCreate::to_empty_form().to_string(),
        "<form id=\"form-id\" class=\"form-class\" style=\"\" method=\"POST\" action=\"\"><input id=\"EarlGrey\" class=\"primary\" style=\"\" name=\"username\" type_=\"text\" value=\"\" placeholder=\"\"/></form>"
    );
}
