use crate::default::model::*;
use async_trait::async_trait;
use mockall::automock;
use sqlx::{Pool, Postgres};
use std::error::Error;

/// UserRepo is a trait that defines the interface for user repository operations.
#[automock]
#[async_trait]
pub trait UserRepo: Send + Sync {
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, Box<dyn Error>>;
}

/// PgUserRepo is a concrete implementation of the UserRepo trait that interacts with a PostgreSQL database.
pub struct PgUserRepo {
    pub pool: Pool<Postgres>,
}

impl PgUserRepo {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait::async_trait]
impl UserRepo for PgUserRepo {
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, Box<dyn Error>> {
        sqlx::query_as::<_, User>("SELECT * FROM t_user WHERE username = $1")
            .bind(username)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }
}
