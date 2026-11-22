use sqlx::FromRow;

#[derive(FromRow)]
pub struct Passphrase {
    pub id: String,
    pub user_id: String,
    pub phrase_salt: String,
    pub phrase_hash: String,
    pub use_mfa: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(FromRow)]
pub struct Session {
    pub id: String,
    pub user_id: String,
    pub trace_id: String,
    pub ip: String,
    pub ua: String,
    pub token: String,
    pub created_at: i64,
    pub updated_at: i64,
}
