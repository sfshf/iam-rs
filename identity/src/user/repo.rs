use crate::user::model::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mockall::automock;
use sqlx::{Pool, Postgres, QueryBuilder};
use std::error::Error;

pub struct UserQueries {
    pub username: Option<String>,
    pub email: Option<String>,
    pub mobile: Option<String>,
    pub status: Option<i32>,
    pub pagination: Option<(i64, i64)>,
    pub ordered_bys: Option<Vec<(String, String)>>,
}

/// UserRepo is a trait that defines the interface for user repository operations.
#[automock]
#[async_trait]
pub trait UserRepo: Send + Sync {
    async fn count_user(&self, queries: UserQueries) -> Result<i64, Box<dyn Error>>;
    async fn get_user_list(&self, queries: UserQueries) -> Result<Vec<User>, Box<dyn Error>>;
    async fn get_user_by_id(&self, id: &str) -> Result<Option<User>, Box<dyn Error>>;
    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, Box<dyn Error>>;
}

/// PgUserRepo is a concrete implementation of the UserRepo trait that interacts with a PostgreSQL database.
#[derive(Clone)]
pub struct PgUserRepo {
    pub pool: Pool<Postgres>,
}

impl PgUserRepo {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// apply_user_filters
    fn apply_user_filters(&self, builder: &mut QueryBuilder<Postgres>, queries: &UserQueries) {
        // where clauses
        if let Some(username) = &queries.username {
            builder
                .push(" AND username ILIKE ")
                .push_bind(format!("%{}%", username));
        }
        if let Some(email) = &queries.email {
            builder
                .push(" AND email ILIKE ")
                .push_bind(format!("%{}%", email));
        }
        if let Some(mobile) = &queries.mobile {
            builder
                .push(" AND phone ILIKE ")
                .push_bind(format!("%{}%", mobile));
        }
        if let Some(status) = &queries.status {
            builder.push(" AND status = ").push_bind(status);
        }
        // orders
        if let Some(ordered_bys) = &queries.ordered_bys {
            let mut ordered_bys = ordered_bys.iter().peekable();
            builder.push(" ORDER BY ");
            while let Some((col, ord)) = ordered_bys.next() {
                builder.push(col).push(" ").push(ord);
                if !ordered_bys.peek().is_none() {
                    builder.push(", ");
                }
            }
        }
        // pagination
        if let Some(pagination) = &queries.pagination {
            let (page, page_size) = pagination;
            builder
                .push(" LIMIT ")
                .push_bind(page_size)
                .push(" OFFSET ")
                .push_bind(page * page_size);
        }
    }
}

#[async_trait::async_trait]
impl UserRepo for PgUserRepo {
    async fn count_user(&self, queries: UserQueries) -> Result<i64, Box<dyn Error>> {
        let mut builder = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM t_user WHERE 1=1");
        self.apply_user_filters(&mut builder, &queries);
        builder
            .build_query_scalar::<i64>()
            .fetch_one(&self.pool)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    async fn get_user_list(&self, queries: UserQueries) -> Result<Vec<User>, Box<dyn Error>> {
        let mut builder = QueryBuilder::<Postgres>::new("SELECT * FROM t_user WHERE 1=1");
        self.apply_user_filters(&mut builder, &queries);
        builder
            .build_query_as::<User>()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    async fn get_user_by_id(&self, id: &str) -> Result<Option<User>, Box<dyn Error>> {
        sqlx::query_as::<_, User>("SELECT * FROM t_user WHERE id = $1::uuid")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    async fn get_user_by_username(&self, username: &str) -> Result<Option<User>, Box<dyn Error>> {
        sqlx::query_as::<_, User>("SELECT * FROM t_user WHERE username = $1")
            .bind(username)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }
}
