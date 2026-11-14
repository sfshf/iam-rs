use sqlx::FromRow;

#[derive(FromRow)]
pub struct User {
    pub id: String,
    pub nickname: String,
    pub username: String,
    pub email: String,
    pub mobile: String,
    pub avatar: String,
    pub status: i8,
    pub created_at: i64,
    pub updated_at: i64,
}
