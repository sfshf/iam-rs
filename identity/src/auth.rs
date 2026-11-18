pub mod auth;
pub mod model;
pub mod repo;

use std::sync::Arc;

use crate::user::repo::UserRepo;
use proto::identity::auth_service_server::AuthService;
use tonic::{Request, Response, Status};

// The AuthServiceImpl struct implements the Auth gRPC service defined in the proto file.
pub struct AuthServiceImpl {
    pub user_repo: Arc<dyn UserRepo>, // user repository
}

impl AuthServiceImpl {
    // Create a new instance of AuthServiceImpl
    pub fn new(user_repo: Arc<dyn UserRepo>) -> Self {
        AuthServiceImpl { user_repo }
    }
}

#[tonic::async_trait]
impl AuthService for AuthServiceImpl {
    async fn set_password(
        &self,
        request: Request<proto::identity::SetPasswordRequest>, // Accept request of type SetPasswordRequest
    ) -> Result<Response<proto::identity::SetPasswordResponse>, Status> {
        auth::set_password(self, request).await
    }

    async fn set_mfa(
        &self,
        request: Request<proto::identity::SetMfaRequest>, // Accept request of type SetMfaRequest
    ) -> Result<Response<proto::identity::SetMfaResponse>, Status> {
        auth::set_mfa(self, request).await
    }

    async fn login(
        &self,
        request: Request<proto::identity::LoginRequest>, // Accept request of type LoginRequest
    ) -> Result<Response<proto::identity::LoginResponse>, Status> {
        auth::login(self, request).await
    }

    async fn logout(
        &self,
        request: Request<proto::identity::LogoutRequest>, // Accept request of type LogoutRequest
    ) -> Result<Response<proto::identity::LogoutResponse>, Status> {
        auth::logout(self, request).await
    }

    async fn refresh_token(
        &self,
        request: Request<proto::identity::RefreshTokenRequest>, // Accept request of type RefreshTokenRequest
    ) -> Result<Response<proto::identity::RefreshTokenResponse>, Status> {
        auth::refresh_token(self, request).await
    }
}
