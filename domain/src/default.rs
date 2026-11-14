pub mod ping;

use proto::domain::domain_service_server::DomainService;
use proto::domain::{PingRequest, PingResponse};
use sqlx::{Pool, Postgres};
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct DomainServiceImpl {
    pub db: Pool<Postgres>, // Database connection pool
}

impl DomainServiceImpl {
    // Create a new instance of DomainServiceImpl
    pub fn new(db: Pool<Postgres>) -> Self {
        DomainServiceImpl { db }
    }
}

#[tonic::async_trait]
impl DomainService for DomainServiceImpl {
    async fn ping(
        &self,
        request: Request<PingRequest>, // Accept request of type PingRequest
    ) -> Result<Response<PingResponse>, Status> {
        ping::ping(self, request).await
    }
}
