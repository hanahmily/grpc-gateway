#![warn(
    missing_debug_implementations,
    rust_2018_idioms,
    unreachable_pub
)]

use proc_macro2::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream};
use prost_build::{Config, Method};
use quote::{ToTokens, TokenStreamExt};

use std::process::Command;
use std::{
    io,
    path::{Path, PathBuf},
};

mod client;
mod server;
mod gateway;

/// Simple `.proto` compiling. Use [`configure`] instead if you need more options.
///
/// The include directory will be the parent folder of the specified path.
/// The package name will be the filename without the extension.
pub fn compile_protos(proto_path: impl AsRef<Path>) -> io::Result<()> {
    let proto_path: &Path = proto_path.as_ref();

    // directory the main .proto file resides in
    let proto_dir = proto_path
        .parent()
        .expect("proto file should reside in a directory");

    let mut config = Config::new();

    let out_dir =  PathBuf::from(std::env::var("OUT_DIR").unwrap());

    config.out_dir(out_dir.clone());
    config.service_generator(Box::new(ServiceGenerator::new()));

    config.compile_protos(&[proto_path], &[proto_dir])?;

    fmt(out_dir.to_str().expect("Expected utf8 out_dir"));

    Ok(())
}

fn fmt(out_dir: &str) {
    let dir = std::fs::read_dir(out_dir).unwrap();

    for entry in dir {
        let file = entry.unwrap().file_name().into_string().unwrap();
        let out = Command::new("rustfmt")
            .arg("--emit")
            .arg("files")
            .arg("--edition")
            .arg("2018")
            .arg(format!("{}/{}", out_dir, file))
            .output()
            .unwrap();

        println!("out: {:?}", out);
        assert!(out.status.success());
    }
}

struct ServiceGenerator {
    clients: TokenStream,
    servers: TokenStream,
    gateways: TokenStream,
}

impl ServiceGenerator {
    fn new() -> Self {
        ServiceGenerator {
            clients: TokenStream::default(),
            servers: TokenStream::default(),
            gateways: TokenStream::default(),
        }
    }
}

impl prost_build::ServiceGenerator for ServiceGenerator {
    fn generate(&mut self, service: prost_build::Service, _buf: &mut String) {
        let path = "super";

        let server = server::generate(&service, path);
        self.servers.extend(server);

        let client = client::generate(&service, path);
        self.clients.extend(client);

        let gateway = gateway::generate(&service, path);
        self.gateways.extend(gateway);
    }

    fn finalize(&mut self, buf: &mut String) {
        if !self.clients.is_empty() {
            let clients = &self.clients;

            let client_service = quote::quote! {
                #clients
            };

            let code = format!("{}", client_service);
            buf.push_str(&code);

            self.clients = TokenStream::default();
        }

        if !self.servers.is_empty() {
            let servers = &self.servers;

            let server_service = quote::quote! {
                #servers
            };

            let code = format!("{}", server_service);
            buf.push_str(&code);

            self.servers = TokenStream::default();
        }

        if !self.gateways.is_empty() {
            let gateways = &self.gateways;

            let gateway = quote::quote! {
                #gateways
            };

            let code = format!("{}", gateway);
            buf.push_str(&code);

            self.gateways = TokenStream::default();
        }
    }
}

// Generate a singular line of a doc comment
fn generate_doc_comment(comment: &str) -> TokenStream {
    let mut doc_stream = TokenStream::new();

    doc_stream.append(Ident::new("doc", Span::call_site()));
    doc_stream.append(Punct::new('=', Spacing::Alone));
    doc_stream.append(Literal::string(&comment));

    let group = Group::new(Delimiter::Bracket, doc_stream);

    let mut stream = TokenStream::new();
    stream.append(Punct::new('#', Spacing::Alone));
    stream.append(group);
    stream
}

// Generate a larger doc comment composed of many lines of doc comments
fn generate_doc_comments<T: AsRef<str>>(comments: &[T]) -> TokenStream {
    let mut stream = TokenStream::new();

    for comment in comments {
        stream.extend(generate_doc_comment(comment.as_ref()));
    }

    stream
}

fn replace_wellknown(proto_path: &str, method: &Method) -> (TokenStream, TokenStream) {
    let request = if method.input_proto_type.starts_with(".google.protobuf") {
        method.input_type.parse::<TokenStream>().unwrap()
    } else {
        syn::parse_str::<syn::Path>(&format!("{}::{}", proto_path, method.input_type))
            .unwrap()
            .to_token_stream()
    };

    let response = if method.output_proto_type.starts_with(".google.protobuf") {
        method.output_type.parse::<TokenStream>().unwrap()
    } else {
        syn::parse_str::<syn::Path>(&format!("{}::{}", proto_path, method.output_type))
            .unwrap()
            .to_token_stream()
    };

    (request, response)
}

fn naive_snake_case(name: &str) -> String {
    let mut s = String::new();
    let mut it = name.chars().peekable();

    while let Some(x) = it.next() {
        s.push(x.to_ascii_lowercase());
        if let Some(y) = it.peek() {
            if y.is_uppercase() {
                s.push('_');
            }
        }
    }

    s
}

#[test]
fn test_snake_case() {
    for case in &[
        ("Service", "service"),
        ("ThatHasALongName", "that_has_a_long_name"),
        ("greeter", "greeter"),
        ("ABCServiceX", "a_b_c_service_x"),
    ] {
        assert_eq!(naive_snake_case(case.0), case.1)
    }
}
