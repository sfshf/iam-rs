pub mod model;
pub mod repo;
pub mod user;

use std::sync::Arc;

use crate::user::repo::UserRepo;
use proto::identity::user_service_server::UserService;
use tonic::{Request, Response, Status};

// The UserServiceImpl struct implements the Identity gRPC service defined in the proto file.
pub struct UserServiceImpl {
    pub user_repo: Arc<dyn UserRepo>, // user repository
}

impl UserServiceImpl {
    // Create a new instance of IdentityServiceImpl
    pub fn new(user_repo: Arc<dyn UserRepo>) -> Self {
        UserServiceImpl { user_repo }
    }
}

#[tonic::async_trait]
impl UserService for UserServiceImpl {
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
}
