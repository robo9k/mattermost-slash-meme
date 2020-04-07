use warp::Filter;

use log::info;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Deserialize, Debug)]
struct Request {
    channel_id: String,
    channel_name: String,
    command: String,
    response_url: Url,
    team_domain: String,
    team_id: String,
    text: String,
    token: String,
    trigger_id: String,
    user_id: String,
    user_name: String,
}

#[derive(Serialize, Debug)]
struct Response {
    text: Option<String>,
}

#[derive(Debug)]
enum Error {
    InvalidToken,
    InvalidAuthorizationHeader,
    InvalidAuthorizationHeaderValue,
}
impl warp::reject::Reject for Error {}

impl From<std::string::FromUtf8Error> for Error {
    fn from(_e: std::string::FromUtf8Error) -> Self {
        Error::InvalidAuthorizationHeader
    }
}

const TOKEN_AUTHORIZATION_SCHEME: &'static str = "Token";

pub fn token_authorization() -> impl Filter<Extract = (String,), Error = warp::Rejection> + Clone {
    warp::header("authorization")
        .map(move |authorization: String| {
            let slice = authorization.as_bytes();
            if slice.starts_with(TOKEN_AUTHORIZATION_SCHEME.as_bytes())
                && slice.len() > TOKEN_AUTHORIZATION_SCHEME.len()
                && slice[TOKEN_AUTHORIZATION_SCHEME.len()] == b' '
            {
                Ok(String::from_utf8(
                    slice[TOKEN_AUTHORIZATION_SCHEME.len() + 1..].to_vec(),
                )?)
            } else {
                Err(Error::InvalidAuthorizationHeaderValue)
            }
        })
        .and_then(|result: Result<_, _>| async { result.map_err(warp::reject::custom) })
        .boxed()
}

pub fn webhook<T>(
    token: String,
) -> impl Clone + std::fmt::Debug + Filter<Extract = (T,), Error = warp::Rejection>
where
    T: 'static + DeserializeOwned + Send,
{
    warp::post()
        .and(token_authorization())
        .and(warp::body::form())
        .map(move |authorization: String, request: T| {
            println!("auth token {}", authorization);
            if authorization == token {
                Ok(request)
            } else {
                Err(Error::InvalidToken)
            }
        })
        .and_then(|result: Result<_, _>| async { result.map_err(warp::reject::custom) })
        .boxed()
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let hook = webhook("3zd39ftkcfnnfrqgb5rie8qtjw".to_string()).map(|request: Request| {
        info!("request: {:?}", request);
        let response = Response {
            text: Some(format!(" hej @{}", request.user_name)),
        };
        warp::reply::json(&response)
    });

    warp::serve(hook).run(([127, 0, 0, 1], 3030)).await;
}
