pub mod ping;

use proto::daily::daily_service_server::DailyService;
use proto::daily::{PingRequest, PingResponse};
use sqlx::{Pool, Postgres};
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct DailyServiceImpl {
    pub db: Pool<sqlx::Postgres>, // Database connection pool
}

impl DailyServiceImpl {
    // Create a new instance of DailyServiceImpl
    pub fn new(db: Pool<Postgres>) -> Self {
        DailyServiceImpl { db }
    }
}

#[tonic::async_trait]
impl DailyService for DailyServiceImpl {
    async fn ping(
        &self,
        request: Request<PingRequest>, // Accept request of type PingRequest
    ) -> Result<Response<PingResponse>, Status> {
        ping::ping(self, request).await
    }
}
