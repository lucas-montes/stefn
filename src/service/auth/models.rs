use chrono::NaiveDateTime;

struct User {
    pk: i64,
    username: String,
    email: String,
    password: String,
    activated_at: Option<NaiveDateTime>,
    created_at: NaiveDateTime,
}

struct Group {
    pk: i64,
    name: String,
}
