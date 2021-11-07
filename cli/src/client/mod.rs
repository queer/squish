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

pub async fn get<S: Into<String>>(route: S) -> Result<String, Box<dyn Error>> {
    request::<S, String>(Method::Get, route, None).await
}

pub async fn post<S: Into<String>, T: Into<String>>(
    route: S,
    body: Option<T>,
) -> Result<String, Box<dyn Error>> {
    request(Method::Post, route, body).await
}

#[allow(dead_code)]
pub async fn put<S: Into<String>, T: Into<String>>(
    route: S,
    body: Option<T>,
) -> Result<String, Box<dyn Error>> {
    request(Method::Put, route, body).await
}

#[allow(dead_code)]
pub async fn patch<S: Into<String>, T: Into<String>>(
    route: S,
    body: Option<T>,
) -> Result<String, Box<dyn Error>> {
    request(Method::Patch, route, body).await
}

#[allow(dead_code)]
pub async fn delete<S: Into<String>, T: Into<String>>(
    route: S,
    body: Option<T>,
) -> Result<String, Box<dyn Error>> {
    request(Method::Delete, route, body).await
}

pub async fn request<S: Into<String>, T: Into<String>>(
    method: Method,
    route: S,
    body: Option<T>,
) -> Result<String, Box<dyn Error>> {
    let url: hyper::http::Uri = Uri::new("/tmp/squishd.sock", &route.into()).into();
    let client = Client::unix();
    let body = match body {
        Some(s) => Body::from(s.into()),
        None => Body::empty(),
    };
    let mut response = match method {
        Method::Get => client.get(url).await?,
        Method::Post => {
            client
                .request(hyper::Request::post(url).body(body)?)
                .await?
        }
        Method::Put => client.request(hyper::Request::put(url).body(body)?).await?,
        Method::Patch => {
            client
                .request(hyper::Request::patch(url).body(body)?)
                .await?
        }
        Method::Delete => {
            client
                .request(hyper::Request::delete(url).body(body)?)
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
    // The server should never send back invalid UTF-8
    Ok(String::from_utf8(body).unwrap())
}
