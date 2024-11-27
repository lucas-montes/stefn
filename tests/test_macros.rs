use stefn::ToForm;

#[test]
fn test_macro() {
    #[derive(ToForm)]
    struct UserCreate {
        username: String,
    }

    let user = UserCreate {
        username: "hey".into(),
    };

    assert_eq!(
        &user.to_form().to_string(),
        "<form id=\"form-id\" class=\"form-class\" style=\"\" method=\"POST\" action=\"\"></form>"
    );
}
