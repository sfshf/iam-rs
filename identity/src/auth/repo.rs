use crate::auth::model::*;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use mockall::automock;
use sqlx::{Pool, Postgres, QueryBuilder};
use std::error::Error;

pub struct SessionQueries {
    pub user_id: Option<String>,
    pub trace_id: Option<String>,
    pub ip: Option<String>,
    pub ua: Option<String>,
    pub token: Option<String>,
    pub pagination: Option<(i64, i64)>,
    pub ordered_bys: Option<Vec<(String, String)>>,
}

/// AuthRepo is a trait that defines the interface for auth repository operations.
#[automock]
#[async_trait]
pub trait AuthRepo: Send + Sync {
    async fn count_session(&self, queries: SessionQueries) -> Result<i64, Box<dyn Error>>;
    async fn get_session_list(
        &self,
        queries: SessionQueries,
    ) -> Result<Vec<Session>, Box<dyn Error>>;
    async fn get_session_by_id(&self, id: &str) -> Result<Option<Session>, Box<dyn Error>>;
}

/// PgAuthRepo is a concrete implementation of the AuthRepo trait that interacts with a PostgreSQL database.
#[derive(Clone)]
pub struct PgAuthRepo {
    pub pool: Pool<Postgres>,
}

impl PgAuthRepo {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }

    /// apply_session_filters
    fn apply_session_filters(
        &self,
        builder: &mut QueryBuilder<Postgres>,
        queries: &SessionQueries,
    ) {
        // where clauses
        if let Some(user_id) = &queries.user_id {
            builder.push(" AND user_id = ").push_bind(user_id);
        }
        if let Some(trace_id) = &queries.trace_id {
            builder.push(" AND trace_id = ").push_bind(trace_id);
            if let Some(ip) = &queries.ip {
                builder.push(" AND ip = ").push_bind(ip);
            }
            if let Some(ua) = &queries.ua {
                builder.push(" AND ua = ").push_bind(ua);
            }
            if let Some(token) = &queries.token {
                builder.push(" AND token = ").push_bind(token);
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
}

#[async_trait::async_trait]
impl AuthRepo for PgAuthRepo {
    async fn count_session(&self, queries: SessionQueries) -> Result<i64, Box<dyn Error>> {
        let mut builder = QueryBuilder::<Postgres>::new("SELECT COUNT(*) FROM t_session WHERE 1=1");
        self.apply_session_filters(&mut builder, &queries);
        builder
            .build_query_scalar::<i64>()
            .fetch_one(&self.pool)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    async fn get_session_list(
        &self,
        queries: SessionQueries,
    ) -> Result<Vec<Session>, Box<dyn Error>> {
        let mut builder = QueryBuilder::<Postgres>::new("SELECT * FROM t_session WHERE 1=1");
        self.apply_session_filters(&mut builder, &queries);
        builder
            .build_query_as::<Session>()
            .fetch_all(&self.pool)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }

    async fn get_session_by_id(&self, id: &str) -> Result<Option<Session>, Box<dyn Error>> {
        sqlx::query_as::<_, Session>("SELECT * FROM t_session WHERE id = $1::uuid")
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| Box::new(e) as Box<dyn Error>)
    }
}
