#[cfg(test)]
mod tests {
    use identity::default::IdentityServiceImpl;
    use identity::default::model::User;
    use identity::default::repo::MockUserRepo;
    use proto::identity::GetUserRequest;
    use proto::identity::identity_service_server::IdentityService;
    use std::sync::Arc;
    use tonic::Request;

    #[tokio::test]
    async fn test_get_user_success() {
        let mut user_repo = MockUserRepo::new();

        user_repo
            .expect_get_user_by_username()
            .with(mockall::predicate::eq("root"))
            .times(1)
            .returning(|_| {
                Ok(Some(User {
                    id: "1".to_string(),
                    nickname: "Alice".to_string(),
                    username: "root".to_string(),
                    email: "".to_string(),
                    mobile: "".to_string(),
                    avatar: "".to_string(),
                    status: 1,
                    created_at: 0,
                    updated_at: 0,
                }))
            });

        let service = IdentityServiceImpl::new(Arc::new(user_repo));

        let resp = service
            .get_user(Request::new(GetUserRequest {
                id: "1".to_string(),
            }))
            .await
            .unwrap();
        let _ = resp.into_inner();
        // assert_eq!(reply, 0);
    }
}
