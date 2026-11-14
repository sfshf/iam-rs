pub mod default;

use clap::Parser;
use proto::aud::aud_service_server::AudServiceServer;
use sqlx::postgres::PgPoolOptions;
use tonic::transport::Server;

#[derive(Parser, Debug)]
#[clap(
    name = "iam-aud",
    version,
    author,
    about = "Audit microservice for IAM system"
)]
struct Args {
    #[clap(long, default_value = "[::1]")]
    ip: String,
    #[clap(short, long, default_value = "50051")]
    port: u16,
    #[clap(
        long,
        default_value = "postgres://postgres:dev123@localhost:5432/iam_aud?sslmode=disable&connect_timeout=5&application_name=iam-aud"
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
    let aud_service = default::AudServiceImpl::new(pool); // Call the new() method to create an instance
    let addr = format!("{}:{}", args.ip, args.port).parse()?;
    Server::builder()
        .add_service(AudServiceServer::new(aud_service))
        .serve(addr)
        .await?;

    Ok(())
}
