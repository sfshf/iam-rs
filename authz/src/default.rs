pub mod ping;
use proto::authz::default_service_server::DefaultService;
use proto::authz::{PingRequest, PingResponse};
use sqlx::{Pool, Postgres};
use tonic::{Request, Response, Status};

#[derive(Debug)]
pub struct AuthzServiceImpl {
    pub db: Pool<Postgres>, // Database connection pool
}

impl AuthzServiceImpl {
    // Create a new instance of AuthzServiceImpl
    pub fn new(db: Pool<Postgres>) -> Self {
        AuthzServiceImpl { db }
    }
}

#[tonic::async_trait]
impl DefaultService for AuthzServiceImpl {
    async fn ping(
        &self,
        request: Request<PingRequest>, // Accept request of type PingRequest
    ) -> Result<Response<PingResponse>, Status> {
        ping::ping(self, request).await
    }
}
