use crate::{naive_snake_case};
use proc_macro2::{TokenStream};
use prost_build::{Method, Service};
use quote::quote;
use quote::ToTokens;
use syn::Ident;

pub(crate) fn generate(service: &Service, proto_path: &str) -> TokenStream {
    let methods = generate_methods(&service, proto_path);

    let gateway_service = quote::format_ident!("{}Gateway", service.name);
    let gateway_mod = quote::format_ident!("{}_gateway", naive_snake_case(&service.name));
    let server_mod = quote::format_ident!("{}_server", naive_snake_case(&service.name));
    let server_trait = quote::format_ident!("{}", &service.name);
    let path = quote::format_ident!("{}", proto_path);

    quote! {
        /// Generated server implementations.
        pub mod #gateway_mod {
            #![allow(unused_variables, dead_code, missing_docs)]

            #[derive(Debug)]
            pub struct #gateway_service(pub tonic::transport::channel::Channel);

            #[tonic::async_trait]
            impl #path::#server_mod::#server_trait for #gateway_service {

                #methods

            }
        }
    }
}

fn generate_methods(service: &Service, proto_path: &str) -> TokenStream {
    let mut stream = TokenStream::new();
    let client_name = format!("{}Client", service.name);
    let client_mod = format!("{}_client", naive_snake_case(&service.name));
    let client_ident = { 
        syn::parse_str::<syn::Path>(&format!("{}::{}::{}", proto_path, client_mod, client_name))
                        .unwrap()
                        .to_token_stream()
    };

    for method in &service.methods {
        let ident = quote::format_ident!("{}", method.name);

        let method_stream = match (method.client_streaming, method.server_streaming) {
            (false, false) => generate_unary(client_ident.clone(), method, ident, proto_path),
            (_, _) => TokenStream::default(),
        };

        stream.extend(method_stream);
    }

    stream
}

fn generate_unary(
    client_ident: TokenStream,
    method: &Method,
    method_ident: Ident,
    proto_path: &str,
) -> TokenStream {
    let (request, response) = crate::replace_wellknown(proto_path, &method);

    quote! {
        async fn #method_ident(
            &self,
            request: tonic::Request<#request>,
        ) -> Result<tonic::Response<#response>, tonic::Status> {
            println!("Got a request from {:?}", request.remote_addr());
            #client_ident::new(self.0.clone())
                .#method_ident(request)
                .await.or_else(|err| Err(tonic::Status::unknown(err.to_string())))
        }
    }
}
