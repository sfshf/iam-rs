
#!/usr/bin/env bash
set -euo pipefail

mkdir -p vendor

# Export the Google APIs protobuf definitions to the vendor directory
buf export buf.build/googleapis/googleapis \
  --output vendor

# Export the gRPC Gateway protobuf definitions to the vendor directory
buf export buf.build/grpc-ecosystem/grpc-gateway \
  --output vendor