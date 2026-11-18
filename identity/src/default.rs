pub mod ping;

use proto::identity::default_service_server::DefaultService;
use proto::identity::{PingRequest, PingResponse};
use tonic::{Request, Response, Status};

// The IdentityServiceImpl struct implements the Identity gRPC service defined in the proto file.
pub struct IdentityServiceImpl {}

impl IdentityServiceImpl {
    // Create a new instance of IdentityServiceImpl
    pub fn new() -> Self {
        IdentityServiceImpl {}
    }
}

#[tonic::async_trait]
impl DefaultService for IdentityServiceImpl {
    // ping is a simple gRPC method that takes a PingRequest and returns a PingResponse.
    // It is used to test the connectivity and responsiveness of the service.
    async fn ping(
        &self,
        request: Request<PingRequest>, // Accept request of type PingRequest
    ) -> Result<Response<PingResponse>, Status> {
        ping::ping(self, request).await
    }
}
