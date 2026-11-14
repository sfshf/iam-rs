use crate::default::AuthzServiceImpl;
use proto::authz::{PingRequest, PingResponse};
use tonic::{Request, Response, Status};

pub async fn ping(
    _service: &AuthzServiceImpl,
    request: Request<PingRequest>, // Accept request of type PingRequest
) -> Result<Response<PingResponse>, Status> {
    // Return an instance of type PingResponse
    println!("Got a request: {:?}", request);

    let reply = PingResponse {
        ts: request.into_inner().ts, // We must use .into_inner() as the fields of gRPC requests and responses are private
    };

    Ok(Response::new(reply)) // Send back our formatted greeting
}
