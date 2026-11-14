pub mod aud {
    tonic::include_proto!("aud.v1"); // The string specified here must match the proto package name
}
pub mod authz {
    tonic::include_proto!("authz.v1"); // The string specified here must match the proto package name
}
pub mod domain {
    tonic::include_proto!("domain.v1"); // The string specified here must match the proto package name
}
pub mod identity {
    tonic::include_proto!("identity.v1"); // The string specified here must match the proto package name
}
// pub mod event {
//     tonic::include_proto!("event.v1"); // The string specified here must match the proto package name
// }
pub mod daily {
    tonic::include_proto!("daily.v1"); // The string specified here must match the proto package name
}
