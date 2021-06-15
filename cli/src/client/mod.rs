use std::error::Error;
use std::vec;

use hyper::Body;
use hyper::{body::HttpBody, Client};
use hyperlocal::{UnixClientExt, Uri};

#[derive(Debug)]
pub enum Method {
    Get,
    Post,
    Put,
    Patch,
    Delete,
}

pub async fn get(route: &'static str) -> Result<String, Box<dyn Error + Send + Sync>> {
    request(Method::Get, route).await
}

pub async fn post(route: &'static str) -> Result<String, Box<dyn Error + Send + Sync>> {
    request(Method::Post, route).await
}

#[allow(dead_code)]
pub async fn put(route: &'static str) -> Result<String, Box<dyn Error + Send + Sync>> {
    request(Method::Put, route).await
}

#[allow(dead_code)]
pub async fn patch(route: &'static str) -> Result<String, Box<dyn Error + Send + Sync>> {
    request(Method::Patch, route).await
}

#[allow(dead_code)]
pub async fn delete(route: &'static str) -> Result<String, Box<dyn Error + Send + Sync>> {
    request(Method::Delete, route).await
}

pub async fn request(
    method: Method,
    route: &'static str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let url: hyper::http::Uri = Uri::new("/tmp/squishd.sock", route).into();
    let client = Client::unix();
    let mut response = match method {
        Method::Get => client.get(url).await?,
        Method::Post => {
            client
                .request(hyper::Request::post(url).body(Body::empty())?)
                .await?
        }
        Method::Put => {
            client
                .request(hyper::Request::put(url).body(Body::empty())?)
                .await?
        }
        Method::Patch => {
            client
                .request(hyper::Request::patch(url).body(Body::empty())?)
                .await?
        }
        Method::Delete => {
            client
                .request(hyper::Request::delete(url).body(Body::empty())?)
                .await?
        }
        #[allow(unreachable_patterns)]
        _ => panic!("unimplemented method: {:?}", method),
    };
    let mut body: Vec<u8> = vec![];
    while let Some(next) = response.data().await {
        let chunk = next?;
        let bytes: Vec<u8> = chunk.to_vec();
        body.extend(&bytes);
        // io::stdout().write_all(&chunk).await?;
    }
    // TODO: This should never panic, but verify anyway
    Ok(String::from_utf8(body).unwrap())
}
