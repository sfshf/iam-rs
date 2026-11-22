#[cfg(test)]
mod tests {
    use identity::user::UserServiceImpl;
    use identity::user::model::User;
    use identity::user::repo::MockUserRepo;
    use proto::identity::GetUserRequest;
    use proto::identity::user_service_server::UserService;
    use std::sync::Arc;
    use tonic::Request;

    #[tokio::test]
    async fn test_get_user_success() {
        let id = "550e8400-e29b-41d4-a716-446655440000";
        let mut user_repo = MockUserRepo::new();
        user_repo
            .expect_get_user_by_id()
            .with(mockall::predicate::eq(id))
            .times(1)
            .returning(|_| {
                Ok(Some(User {
                    id: id.to_string(),
                    username: "root".to_string(),
                    email: "".to_string(),
                    mobile: "".to_string(),
                    status: 1,
                    created_at: 0,
                    updated_at: 0,
                }))
            });
        let service = UserServiceImpl::new(Arc::new(user_repo));
        let resp = service
            .get_user(Request::new(GetUserRequest { id: id.to_string() }))
            .await
            .unwrap();
        let reply = resp.into_inner();
        assert_eq!(reply.id, id);
    }
}
