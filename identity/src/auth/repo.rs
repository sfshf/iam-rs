use crate::user::model::*;
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
    async fn get_session_list(&self, queries: SessionQueries) -> Result<Vec<User>, Box<dyn Error>>;
    async fn get_session_by_id(&self, id: &str) -> Result<Option<User>, Box<dyn Error>>;
}
