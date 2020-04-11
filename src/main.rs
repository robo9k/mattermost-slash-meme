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
    skip_slack_parsing: Option<bool>,
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

struct MemeRequest {
    meme: String,
    boxes: Vec<String>,
}

fn usage(slash_command: String) -> Response {
    let usage_response = Response {
		text: Some(format!("Usage: `{slash_command} <id>⇧⏎<text>⇧⏎…`\nExample:\n```{slash_command} 181913649\nmaking memes yourself\nusing a bot to make memes```", slash_command=slash_command)),
		response_type: None,
		username: None,
		channel_id: None,
		icon_url: Some(Url::parse("https://imgflip.com/imgflip_white_96.png").unwrap()),
        goto_location: None,
		skip_slack_parsing: Some(true),
	};
    info!(
        "usage response {:?}",
        serde_json::to_string(&usage_response)
    );
    usage_response
}

async fn meme_reply(
    imgflip: std::sync::Arc<imgflip::AccountClient>,
    request: Request,
) -> Result<impl warp::Reply, warp::Rejection> {
    info!("request: {:?}", request);

    let mut text_lines = request.text.lines();
    let meme = match text_lines.next() {
        Some(meme) => meme.to_string(),
        None => {
            let usage_response = usage(request.command);
            return Ok(warp::reply::json(&usage_response));
        }
    };
    let boxes: Vec<_> = text_lines.map(|s| s.to_string()).collect();
    if boxes.is_empty() {
        let usage_response = usage(request.command);
        return Ok(warp::reply::json(&usage_response));
    }

    let meme_request = MemeRequest { meme, boxes };

    let response = Response {
        text: Some("working on it".to_string()),
        response_type: None,
        username: None,
        channel_id: None,
        icon_url: Some(Url::parse("https://imgflip.com/imgflip_white_96.png").unwrap()),
        goto_location: None,
        skip_slack_parsing: Some(true),
    };
    info!("response {:?}", response);

    tokio::spawn(reply_with_meme(imgflip, meme_request, request.response_url));

    Ok(warp::reply::json(&response))
}

async fn reply_with_meme(
    imgflip: std::sync::Arc<imgflip::AccountClient>,
    meme_request: MemeRequest,
    response_url: Url,
) {
    let mut meme_caption = imgflip::CaptionBoxesRequestBuilder::new(meme_request.meme);
    for b in meme_request.boxes.iter() {
        meme_caption = meme_caption.caption_box(imgflip::CaptionBoxBuilder::new(b).build());
    }
    let meme_caption = meme_caption.build();
    info!("caption {:?}", meme_caption);

    let meme_response = imgflip.caption_image(meme_caption).await;
    info!("meme_response {:?}", meme_response);
    let user_response = match meme_response {
        Ok(meme) => Response {
            text: Some(format!(" hej {}", meme.url())),
            response_type: Some("in_channel".to_string()),
            username: None,
            channel_id: None,
            icon_url: Some(Url::parse("https://imgflip.com/imgflip_white_96.png").unwrap()),
            goto_location: Some(meme.page_url().clone()),
            skip_slack_parsing: Some(true),
        },
        Err(error) => match error {
            imgflip::Error::ApiError(error_message) => Response {
                text: Some(format!("Uhoh, something went wrong: {}", error_message)),
                response_type: None,
                username: None,
                channel_id: None,
                icon_url: Some(Url::parse("https://imgflip.com/imgflip_white_96.png").unwrap()),
                goto_location: None,
                skip_slack_parsing: Some(true),
            },
            _ => Response {
                text: Some("Uhoh, something went wrong".to_string()),
                response_type: None,
                username: None,
                channel_id: None,
                icon_url: Some(Url::parse("https://imgflip.com/imgflip_white_96.png").unwrap()),
                goto_location: None,
                skip_slack_parsing: Some(true),
            },
        },
    };
    info!("user_response {:?}", user_response);

    let client = reqwest::Client::new();
    let res = client
        .post(response_url)
        .json(&user_response)
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
