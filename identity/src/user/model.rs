use sqlx::FromRow;

#[derive(FromRow)]
pub struct User {
    pub id: String,
    pub username: String,
    pub email: String,
    pub mobile: String,
    pub status: i32,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(FromRow)]
pub struct UserProfile {
    pub id: String,
    pub user_id: String,
    pub nickname: String,
    pub avatar: String,
    pub real_name: String,
    pub gender: i32,
    pub birthday: String,
    pub address: String,
    pub created_at: i64,
    pub updated_at: i64,
}
