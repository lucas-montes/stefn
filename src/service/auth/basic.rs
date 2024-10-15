pub fn hash_password(password: &str) {}
// pub fn hash_password(password: &str) -> Result<String, AppError> {
//     let salt = SaltString::generate(&mut OsRng);
//     Ok(Argon2::default()
//         .hash_password(password.as_bytes(), &salt)
//         .map_err(AppError::ErrorHashingPassword)?
//         .to_string())
// }
pub fn verify_password(raw_password: &str, db_password: &str) {}
// pub fn verify_password(raw_password: &str, db_password: &str) {
//     let parsed_hash = PasswordHash::new(db_password).map_err(AppError::ErrorHashingPassword)?;
//     Argon2::default()
//         .verify_password(raw_password.as_bytes(), &parsed_hash)
//         .map_err(AppError::WrongPassword)?
// }
