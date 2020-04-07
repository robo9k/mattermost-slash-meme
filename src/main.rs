use warp::Filter;

use serde::{Deserialize,Serialize};
use log::{info};
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

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let hook = warp::post()
        .and(warp::body::form())
        .map(|request: Request| {
		info!("request: {:?}", request);
		let response = Response {
			text: Some("foo".to_string()),
		};
		warp::reply::json(&response)
		});

    warp::serve(hook).run(([127, 0, 0, 1], 3030)).await;
}
