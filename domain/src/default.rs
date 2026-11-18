pub mod ping;

use proto::domain::default_service_server::DefaultService;
use proto::domain::{PingRequest, PingResponse};
use sqlx::{Pool, Postgres};
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct DefaultServiceImpl {
    pub db: Pool<Postgres>, // Database connection pool
}

impl DefaultServiceImpl {
    // Create a new instance of DefaultServiceImpl
    pub fn new(db: Pool<Postgres>) -> Self {
        DefaultServiceImpl { db }
    }
}

#[tonic::async_trait]
impl DefaultService for DefaultServiceImpl {
    async fn ping(
        &self,
        request: Request<PingRequest>, // Accept request of type PingRequest
    ) -> Result<Response<PingResponse>, Status> {
        ping::ping(self, request).await
    }
}
