pub mod model;
pub mod ping;
pub mod repo;
pub mod user;

use std::sync::Arc;

use proto::identity::identity_service_server::IdentityService;
use proto::identity::{PingRequest, PingResponse};
use tonic::{Request, Response, Status};

use crate::default::repo::UserRepo;

// The IdentityServiceImpl struct implements the Identity gRPC service defined in the proto file.
pub struct IdentityServiceImpl {
    pub user_repo: Arc<dyn UserRepo>, // user repository
}

impl IdentityServiceImpl {
    // Create a new instance of IdentityServiceImpl
    pub fn new(user_repo: Arc<dyn UserRepo>) -> Self {
        IdentityServiceImpl { user_repo }
    }
}

#[tonic::async_trait]
impl IdentityService for IdentityServiceImpl {
    // ping is a simple gRPC method that takes a PingRequest and returns a PingResponse.
    // It is used to test the connectivity and responsiveness of the service.
    async fn ping(
        &self,
        request: Request<PingRequest>, // Accept request of type PingRequest
    ) -> Result<Response<PingResponse>, Status> {
        ping::ping(self, request).await
    }
    // get_user is a gRPC method that takes a GetUserRequest and returns a GetUserResponse.
    // It is used to retrieve user information based on the provided criteria.
    async fn get_user(
        &self,
        request: Request<proto::identity::GetUserRequest>, // Accept request of type GetUserRequest
    ) -> Result<Response<proto::identity::GetUserResponse>, Status> {
        user::get_user(self, request).await
    }

    async fn add_user(
        &self,
        request: Request<proto::identity::AddUserRequest>, // Accept request of type AddUserRequest
    ) -> Result<Response<proto::identity::AddUserResponse>, Status> {
        user::add_user(self, request).await
    }

    async fn update_user(
        &self,
        request: Request<proto::identity::UpdateUserRequest>, // Accept request of type UpdateUserRequest
    ) -> Result<Response<proto::identity::UpdateUserResponse>, Status> {
        user::update_user(self, request).await
    }

    async fn delete_user(
        &self,
        request: Request<proto::identity::DeleteUserRequest>, // Accept request of type DeleteUserRequest
    ) -> Result<Response<proto::identity::DeleteUserResponse>, Status> {
        user::delete_user(self, request).await
    }

    async fn list_user(
        &self,
        request: Request<proto::identity::ListUserRequest>, // Accept request of type ListUserRequest
    ) -> Result<Response<proto::identity::ListUserResponse>, Status> {
        user::list_user(self, request).await
    }

    async fn login(
        &self,
        request: Request<proto::identity::LoginRequest>, // Accept request of type LoginRequest
    ) -> Result<Response<proto::identity::LoginResponse>, Status> {
        user::login(self, request).await
    }

    async fn logout(
        &self,
        request: Request<proto::identity::LogoutRequest>, // Accept request of type LogoutRequest
    ) -> Result<Response<proto::identity::LogoutResponse>, Status> {
        user::logout(self, request).await
    }
}
