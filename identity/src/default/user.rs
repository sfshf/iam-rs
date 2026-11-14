use crate::default::IdentityServiceImpl;
use proto::identity::{
    AddUserRequest, AddUserResponse, GetUserRequest, GetUserResponse, LoginRequest, LoginResponse,
    LogoutRequest, LogoutResponse,
};
use tonic::{Request, Response, Status};

// Handle get_user requests
pub async fn get_user(
    service: &IdentityServiceImpl,
    request: Request<GetUserRequest>, // Accept request of type GetUserRequest
) -> Result<Response<GetUserResponse>, Status> {
    // 从数据库检索用户名为'root'的账号，且该账号已被设置邮箱
    service
        .user_repo
        .get_user_by_username(&request.into_inner().id)
        .await
        .map_err(|e| match e.downcast_ref::<sqlx::Error>() {
            Some(sqlx::Error::RowNotFound) => {
                Status::not_found("Root account not found or email not set")
            }
            _ => Status::internal(format!("Database error: {}", e)),
        })?;
    Ok(Response::new(GetUserResponse {
        id: "1".to_string(),
        username: "root".to_string(),
        email: "".to_string(),
        mobile: "".to_string(),
        status: 1,
        created_at: 0,
        updated_at: 0,
    })) // Send back our formatted reply
}

pub async fn add_user(
    service: &IdentityServiceImpl,
    request: Request<AddUserRequest>, // Accept request of type AddUserRequest
) -> Result<Response<AddUserResponse>, Status> {
    // Implementation for adding a new user
    Ok(Response::new(AddUserResponse {}))
}

pub async fn update_user(
    service: &IdentityServiceImpl,
    request: Request<proto::identity::UpdateUserRequest>, // Accept request of type UpdateUserRequest
) -> Result<Response<proto::identity::UpdateUserResponse>, Status> {
    // Implementation for updating user information
    Ok(Response::new(proto::identity::UpdateUserResponse {}))
}

pub async fn delete_user(
    service: &IdentityServiceImpl,
    request: Request<proto::identity::DeleteUserRequest>, // Accept request of type DeleteUserRequest
) -> Result<Response<proto::identity::DeleteUserResponse>, Status> {
    // Implementation for deleting a user
    Ok(Response::new(proto::identity::DeleteUserResponse {}))
}

pub async fn list_user(
    service: &IdentityServiceImpl,
    request: Request<proto::identity::ListUserRequest>, // Accept request of type ListUserRequest
) -> Result<Response<proto::identity::ListUserResponse>, Status> {
    // Implementation for listing users
    Ok(Response::new(proto::identity::ListUserResponse {
        total: 0,
        users: Vec::new(),
    }))
}

pub async fn login(
    service: &IdentityServiceImpl,
    request: Request<LoginRequest>, // Accept request of type LoginRequest
) -> Result<Response<LoginResponse>, Status> {
    // Implementation for user login
    Ok(Response::new(LoginResponse {
        domains: Vec::new(),
    }))
}

pub async fn logout(
    service: &IdentityServiceImpl,
    request: Request<LogoutRequest>, // Accept request of type LogoutRequest
) -> Result<Response<LogoutResponse>, Status> {
    // Implementation for user logout
    Ok(Response::new(LogoutResponse {}))
}
