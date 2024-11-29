use stefn::forms::ToForm;
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
        "<form id=\"form-id\" class=\"form-class\" style=\"\" method=\"POST\" action=\"\"><div id=\"EarlGrey\" class=\"primary\" style=\"\"><label id=\"EarlGrey\" class=\"primary\" style=\"\">username</label><input id=\"EarlGrey\" class=\"primary\" style=\"\" name=\"username\" type_=\"text\" value=\"hey\" placeholder=\"\"/></div></form>"
    );

    assert_eq!(
        &UserCreate::to_empty_form().to_string(),
        "<form id=\"form-id\" class=\"form-class\" style=\"\" method=\"POST\" action=\"\"><div id=\"EarlGrey\" class=\"primary\" style=\"\"><label id=\"EarlGrey\" class=\"primary\" style=\"\">username</label><input id=\"EarlGrey\" class=\"primary\" style=\"\" name=\"username\" type_=\"text\" value=\"\" placeholder=\"\"/></div></form>"
    );
}

#[test]
fn test_macro_full() {
    #[derive(ToForm)]
    struct UserCreate {
        #[html(id = "EarlGrey", class = "primary")]
        username: String,
        age: f64,
    }

    let user = UserCreate {
        username: "hey".into(),
        age: 1.0,
    };

    assert_eq!(
        &user.to_form().to_string(),
        "<form id=\"form-id\" class=\"form-class\" style=\"\" method=\"POST\" action=\"\"><div id=\"EarlGrey\" class=\"primary\" style=\"\"><label id=\"EarlGrey\" class=\"primary\" style=\"\">username</label><input id=\"EarlGrey\" class=\"primary\" style=\"\" name=\"username\" type_=\"text\" value=\"hey\" placeholder=\"\"/></div></form>"
    );
}
