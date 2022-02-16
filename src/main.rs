use std::{collections::VecDeque, convert::Infallible, io::Read, net::IpAddr, sync::Arc};

use chrono::{DateTime, Local};
use chrono_humanize::HumanTime;
use tokio::sync::RwLock;
use warp::{hyper::StatusCode, Buf, Filter, Rejection, Reply};

const PING_COUNT: usize = 8;

struct CanaryPing {
    reason: String,
    time: DateTime<Local>,
}

struct CanaryState {
    pings: RwLock<VecDeque<CanaryPing>>,
}

impl Default for CanaryState {
    fn default() -> Self {
        Self {
            pings: RwLock::new(VecDeque::with_capacity(PING_COUNT)),
        }
    }
}

fn with_state(
    state: Arc<CanaryState>,
) -> impl Filter<Extract = (Arc<CanaryState>,), Error = Infallible> + Clone {
    warp::any().map(move || state.clone())
}

async fn status_page(state: Arc<CanaryState>) -> Result<impl Reply, Rejection> {
    let status_entries = state
        .pings
        .read()
        .await
        .iter()
        .map(|ping| {
            let reason = &ping.reason;
            let ago = HumanTime::from(ping.time.signed_duration_since(Local::now()));
            let utc_time = ping.time.naive_utc();

            format!(r#"<li>{reason} - <time datetime="{utc_time}">{ago}</time></li>"#,)
        })
        .collect::<Vec<String>>()
        .join("\n    ");

    Ok(warp::reply::html(format!(
        r#"<!DOCTYPE html>
<meta charset="utf-8">
<title>presence canary</title>
<style>
    body {{
        max-width: 960px;
        font-family: sans-serif;
        font-size: 1.25em;
        margin: 0 auto;
    }}
</style>

<h1>presence canary</h1>
<p>known pings (up to {PING_COUNT}):</p>
<ol>
    {status_entries}
</ol>"#
    )))
}

async fn handle_ping(
    operator_token: Arc<String>,
    state: Arc<CanaryState>,
    authorization: String,
    body: impl Buf,
) -> Result<impl Reply, Rejection> {
    if authorization != format!("Bearer {operator_token}") {
        return Ok(warp::reply::with_status(
            "Unauthorized",
            StatusCode::UNAUTHORIZED,
        ));
    }

    let mut reason = String::new();
    let _ = body.reader().read_to_string(&mut reason).unwrap();

    let mut pings = state.pings.write().await;
    pings.push_front(CanaryPing {
        reason,
        time: Local::now(),
    });
    pings.truncate(PING_COUNT);

    Ok(warp::reply::with_status("Ok", StatusCode::OK))
}

#[tokio::main]
async fn main() {
    let operator_token =
        Arc::new(std::env::var("OPERATOR_TOKEN").expect("OPERATOR_TOKEN should be defined"));

    let state = Arc::new(CanaryState::default());

    let status_route = warp::get()
        .and(with_state(state.clone()))
        .and_then(status_page);

    let ping_route = warp::post()
        .map(move || operator_token.clone())
        .and(with_state(state.clone()))
        .and(warp::header::<String>("Authorization"))
        .and(warp::body::aggregate())
        .and_then(handle_ping);

    let routes = status_route.or(ping_route);

    let ip = std::env::var("IP")
        .ok()
        .and_then(|s| s.parse::<IpAddr>().ok())
        .unwrap_or_else(|| [127, 0, 0, 1].into());
    let port = std::env::var("PORT")
        .ok()
        .and_then(|s| s.parse::<u16>().ok())
        .unwrap_or(3000);

    println!("Listening at http://{}:{} ...", &ip, &port);
    warp::serve(routes).run((ip, port)).await;
}
