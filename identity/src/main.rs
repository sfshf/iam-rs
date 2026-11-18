use std::sync::Arc;

use clap::Parser;
use identity::{
    auth::AuthServiceImpl,
    default::IdentityServiceImpl,
    user::{UserServiceImpl, repo::PgUserRepo},
};
use proto::identity::{
    auth_service_server::AuthServiceServer, default_service_server::DefaultServiceServer,
    user_service_server::UserServiceServer,
};
use sqlx::postgres::PgPoolOptions;
use tonic::transport::Server;

#[derive(Parser, Debug)]
#[clap(
    name = "iam-identity",
    version,
    author,
    about = "Identity microservice for IAM system"
)]
struct Args {
    #[clap(long, default_value = "[::1]")]
    ip: String,
    #[clap(short, long, default_value = "50051")]
    port: u16,
    #[clap(
        long,
        default_value = "postgres://postgres:dev123@localhost:5432/iam_identity?sslmode=disable&connect_timeout=5&application_name=iam-identity"
    )]
    pg_url: String,
    #[clap(long, default_value = "5")]
    pg_max_connections: u32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    // relational database connection pool
    let user_repo = PgUserRepo::new(
        PgPoolOptions::new()
            .max_connections(args.pg_max_connections)
            .connect(&args.pg_url)
            .await?,
    );
    let identity_service = IdentityServiceImpl::new(); // Call the new() method to create an instance
    let user_service = UserServiceImpl::new(Arc::new(user_repo.clone())); // Call the new() method to create an instance    
    let auth_service = AuthServiceImpl::new(Arc::new(user_repo)); // Call the new() method to create an instance
    let addr = format!("{}:{}", args.ip, args.port)
        .parse()
        .expect("Invalid postgres server address");
    Server::builder()
        .add_service(DefaultServiceServer::new(identity_service))
        .add_service(UserServiceServer::new(user_service))
        .add_service(AuthServiceServer::new(auth_service))
        .serve(addr)
        .await?;

    Ok(())
}
