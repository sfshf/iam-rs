pub mod default;

use crate::default::AuthzServiceImpl;
use clap::Parser;
use proto::authz::authz_service_server::AuthzServiceServer;
use sqlx::postgres::PgPoolOptions;
use tonic::transport::Server;

#[derive(Parser, Debug)]
#[clap(
    name = "iam-authz",
    version,
    author,
    about = "Authorization microservice for IAM system"
)]
struct Args {
    #[clap(long, default_value = "[::1]")]
    ip: String,
    #[clap(short, long, default_value = "50051")]
    port: u16,
    #[clap(
        long,
        default_value = "postgres://postgres:dev123@localhost:5432/iam_authz?sslmode=disable&connect_timeout=5&application_name=iam-authz"
    )]
    pg_url: String,
    #[clap(long, default_value = "5")]
    pg_max_connections: u32,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    // relational database connection pool
    let pool = PgPoolOptions::new()
        .max_connections(args.pg_max_connections)
        .connect(&args.pg_url)
        .await?;
    let authz_service = AuthzServiceImpl::new(pool); // Call the new() method to create an instance
    let addr = format!("{}:{}", args.ip, args.port).parse()?;

    Server::builder()
        .add_service(AuthzServiceServer::new(authz_service))
        .serve(addr)
        .await?;

    Ok(())
}
