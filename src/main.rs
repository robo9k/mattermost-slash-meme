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
    response_type: Option<String>,
    username: Option<String>,
    channel_id: Option<String>,
    icon_url: Option<Url>,
    goto_location: Option<Url>,
}

#[derive(Debug)]
enum Error {
    InvalidToken,
    InvalidAuthorizationHeader,
    InvalidAuthorizationHeaderValue,
    ImgFlip,
}
impl warp::reject::Reject for Error {}

impl From<std::string::FromUtf8Error> for Error {
    fn from(_e: std::string::FromUtf8Error) -> Self {
        Error::InvalidAuthorizationHeader
    }
}

impl From<imgflip::Error> for Error {
    fn from(_e: imgflip::Error) -> Self {
        Error::ImgFlip
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

pub fn webhook<F, T>(
    token_validator: F,
) -> impl Clone + std::fmt::Debug + Filter<Extract = (T,), Error = warp::Rejection>
where
    F: 'static + Fn(&str) -> bool + Clone + Send + Sync,
    T: 'static + DeserializeOwned + Send,
{
    warp::post()
        .and(token_authorization())
        .and(warp::body::form())
        .map(move |authorization: String, request: T| {
            println!("auth token {}", authorization);
            if token_validator(&authorization) {
                Ok(request)
            } else {
                Err(Error::InvalidToken)
            }
        })
        .and_then(|result: Result<_, _>| async { result.map_err(warp::reject::custom) })
        .boxed()
}

async fn meme_reply(
    imgflip: std::sync::Arc<imgflip::AccountClient>,
    request: Request,
) -> Result<impl warp::Reply, warp::Rejection> {
    info!("request: {:?}", request);

    let response = Response {
        text: Some("working on it".to_string()),
        response_type: None,
        username: None,
        channel_id: None,
        icon_url: Some(Url::parse("https://imgflip.com/imgflip_white_96.png").unwrap()),
        goto_location: None,
    };
    info!("response {:?}", response);

    tokio::spawn(reply_with_meme(imgflip, request.text, request.response_url));

    Ok(warp::reply::json(&response))
}

async fn reply_with_meme(
    imgflip: std::sync::Arc<imgflip::AccountClient>,
    text: String,
    response_url: Url,
) {
    let meme_caption = imgflip::CaptionBoxesRequestBuilder::new("61580")
        .caption_box(imgflip::CaptionBoxBuilder::new(text).build())
        .build();
    info!("caption {:?}", meme_caption);

    let meme = imgflip.caption_image(meme_caption).await.unwrap();
    info!("meme {:?}", meme);

    let response = Response {
        text: Some(format!(" hej {}", meme.url())),
        response_type: Some("in_channel".to_string()),
        username: None,
        channel_id: None,
        icon_url: Some(Url::parse("https://imgflip.com/imgflip_white_96.png").unwrap()),
        goto_location: Some(meme.page_url().clone()),
    };
    info!("response {:?}", response);

    let client = reqwest::Client::new();
    let res = client
        .post(response_url)
        .json(&response)
        .send()
        .await
        .unwrap();
}

fn with_imgflip(
    imgflip: std::sync::Arc<imgflip::AccountClient>,
) -> impl Filter<Extract = (std::sync::Arc<imgflip::AccountClient>,), Error = std::convert::Infallible>
       + Clone {
    warp::any().map(move || imgflip.clone())
}

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let imgflip = std::sync::Arc::new(imgflip::AccountClient::new("freeforall6", "nsfw1234"));

    let hook = with_imgflip(imgflip)
        .and(webhook(|token| "3zd39ftkcfnnfrqgb5rie8qtjw" == token))
        .and_then(meme_reply);

    warp::serve(hook).run(([127, 0, 0, 1], 3030)).await;
}
