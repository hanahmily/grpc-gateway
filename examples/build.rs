fn main() {
    grpc_gateway_build::compile_protos("proto/helloworld/helloworld.proto").unwrap();
}