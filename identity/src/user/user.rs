use crate::user::UserServiceImpl;
use proto::identity::{AddUserRequest, AddUserResponse, GetUserRequest, GetUserResponse};
use tonic::{Request, Response, Status};

pub const ROOT_USER_NAME: &str = "root"; // 硬编码唯一的root账号的用户名；绝不允许有同名的其他账号

// Handle get_user requests
pub async fn get_user(
    service: &UserServiceImpl,
    request: Request<GetUserRequest>, // Accept request of type GetUserRequest
) -> Result<Response<GetUserResponse>, Status> {
    let user = service
        .user_repo
        .get_user_by_id(&request.into_inner().id)
        .await
        .map_err(|e| match e.downcast_ref::<sqlx::Error>() {
            Some(sqlx::Error::RowNotFound) => Status::not_found("target user not found"),
            _ => Status::internal(format!("Database error: {}", e)),
        })?;
    let Some(user) = user else {
        return Err(Status::not_found("target user not found"));
    };
    // Send back our formatted reply
    Ok(Response::new(GetUserResponse {
        id: user.id,
        username: user.username,
        email: user.email,
        mobile: user.mobile,
        status: user.status,
        created_at: user.created_at,
        updated_at: user.updated_at,
    }))
}

// Handle add_user requests
pub async fn add_user(
    service: &UserServiceImpl,
    request: Request<AddUserRequest>, // Accept request of type AddUserRequest
) -> Result<Response<AddUserResponse>, Status> {
    Ok(Response::new(AddUserResponse {}))
}

pub async fn update_user(
    service: &UserServiceImpl,
    request: Request<proto::identity::UpdateUserRequest>, // Accept request of type UpdateUserRequest
) -> Result<Response<proto::identity::UpdateUserResponse>, Status> {
    // Implementation for updating user information
    Ok(Response::new(proto::identity::UpdateUserResponse {}))
}

pub async fn delete_user(
    service: &UserServiceImpl,
    request: Request<proto::identity::DeleteUserRequest>, // Accept request of type DeleteUserRequest
) -> Result<Response<proto::identity::DeleteUserResponse>, Status> {
    // Implementation for deleting a user
    Ok(Response::new(proto::identity::DeleteUserResponse {}))
}

pub async fn list_user(
    service: &UserServiceImpl,
    request: Request<proto::identity::ListUserRequest>, // Accept request of type ListUserRequest
) -> Result<Response<proto::identity::ListUserResponse>, Status> {
    // Implementation for listing users
    Ok(Response::new(proto::identity::ListUserResponse {
        total: 0,
        users: Vec::new(),
    }))
}
