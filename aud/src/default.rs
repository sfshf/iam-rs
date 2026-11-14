pub mod ping;

use proto::aud::aud_service_server::AudService;
use proto::aud::{PingRequest, PingResponse};
use sqlx::{Pool, Postgres};
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct AudServiceImpl {
    pub db: Pool<Postgres>, // Database connection pool
}

impl AudServiceImpl {
    // Create a new instance of AudServiceImpl
    pub fn new(db: Pool<sqlx::Postgres>) -> Self {
        AudServiceImpl { db }
    }
}

#[tonic::async_trait]
impl AudService for AudServiceImpl {
    async fn ping(
        &self,
        request: Request<PingRequest>, // Accept request of type PingRequest
    ) -> Result<Response<PingResponse>, Status> {
        ping::ping(self, request).await
    }
}
