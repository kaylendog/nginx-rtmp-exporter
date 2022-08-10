mod context;
mod meta;
mod metrics;
mod xml;

use std::{
    convert::Infallible,
    env,
    error::Error,
    net::{IpAddr, SocketAddr},
    path::PathBuf,
    sync::Arc,
};

use clap::Parser;
use dotenv::dotenv;
use meta::Format;
use prometheus::{Encoder, TextEncoder};
use reqwest::Url;
use serde::Serialize;
use tokio::sync::Mutex;
use tracing::{debug, error, info};
use tracing_subscriber::fmt::format::FmtSpan;
use warp::{
    http::HeaderValue,
    hyper::{header::CONTENT_TYPE, Body, StatusCode},
    Filter, Rejection, Reply,
};

use crate::{context::Context, meta::MetaProvider, metrics::collect_metrics};

/// Prometheus data exporter for NGINX servers running the nginx-rtmp-module.
#[derive(Parser)]
struct Args {
    /// The RTMP statistics endpoint of NGINX.
    #[clap(long)]
    pub scrape_url: Url,
    /// The host to listen on.
    #[clap(default_value = "127.0.0.1", long)]
    pub host: IpAddr,
    /// The port to listen on.
    #[clap(default_value = "9114", short, long)]
    pub port: u16,
    /// An optional path to a metadata file.
    #[clap(long)]
    pub metadata: Option<PathBuf>,
    /// An optional format for the metadata file.
    #[clap(long, default_value = "json")]
    pub format: Format,
}

fn encode_metrics() -> Result<(TextEncoder, String), Box<dyn Error>> {
    let encoder = TextEncoder::new();
    let mut buf = String::new();
    // gather and encode metrics
    let metric_families = prometheus::gather();
    encoder.encode_utf8(&metric_families, &mut buf)?;
    // return encoder and buffer
    Ok((encoder, buf))
}

/// An API error serializable to JSON.
#[derive(Serialize)]
struct ErrorMessage {
    code: u16,
    message: String,
}

async fn handle_rejection(err: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    if err.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "NOT_FOUND";
    } else if let Some(e) = err.find::<warp::filters::body::BodyDeserializeError>() {
        message = match e.source() {
            Some(cause) => {
                if cause.to_string().contains("denom") {
                    "FIELD_ERROR: denom"
                } else {
                    "BAD_REQUEST"
                }
            }
            None => "BAD_REQUEST",
        };
        code = StatusCode::BAD_REQUEST;
    } else if err.find::<warp::reject::MethodNotAllowed>().is_some() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "METHOD_NOT_ALLOWED";
    } else {
        error!("unhandled rejection: {:?}", err);
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "UNHANDLED_REJECTION";
    };

    let json = warp::reply::json(&ErrorMessage { code: code.as_u16(), message: message.into() });

    Ok(warp::reply::with_status(json, code))
}

#[tokio::main]
async fn main() {
    let args: Args = Args::parse();
    // intialize tracing
    let filter =
        env::var("RUST_LOG").unwrap_or_else(|_| "warn,nginx_rtmp_exporter=info".to_owned());
    tracing_subscriber::fmt().with_env_filter(filter).with_span_events(FmtSpan::CLOSE).init();
    // print splash
    info!("{} v{}", env!("CARGO_PKG_NAME"), env!("VERGEN_GIT_SEMVER"));
    // print version information
    debug!(
        build_timestamp = env!("VERGEN_BUILD_TIMESTAMP"),
        rustc_version = env!("VERGEN_RUSTC_SEMVER"),
        builder_host = env!("VERGEN_RUSTC_HOST_TRIPLE")
    );
    // load dotenv if in dev env
    if cfg!(debug_assertions) {
        dotenv().ok();
    }
    // load metadata
    let provider = match args.metadata {
        Some(path) => {
            let provider =
                MetaProvider::from_file(&path, args.format).expect("Failed to load metadata");
            info!("Loaded metadata from {:?}", path);
            provider
        }
        None => MetaProvider::default(),
    };
    // create threadsafe context
    let ctx = Context::new(args.scrape_url, provider);
    let ctx = Arc::new(Mutex::new(ctx));
    // create context filter
    let ctx = warp::any().map(move || ctx.clone());
    // create index filter
    let index = warp::get()
        .and(warp::path!("metrics"))
        .and(warp::path::end())
        .and(ctx)
        .then(|ctx: Arc<Mutex<Context>>| async move {
            let mut ctx = ctx.lock().await;
            collect_metrics(&mut ctx).await;
            encode_metrics()
        })
        .map(|res: Result<(TextEncoder, String), Box<dyn Error>>| match res {
            Ok((encoder, buf)) => {
                let mut res = warp::reply::Response::new(Body::from(buf));
                res.headers_mut()
                    .insert(CONTENT_TYPE, HeaderValue::from_str(encoder.format_type()).unwrap());
                res
            }
            Err(err) => {
                error!("Failed to collect metrics");
                error!("{}", err);
                warp::reply::with_status(warp::reply(), StatusCode::INTERNAL_SERVER_ERROR)
                    .into_response()
            }
        })
        .recover(handle_rejection)
        .with(warp::trace::request())
        .with(warp::log("nginx_rtmp_exporter"));
    // get address and listen
    let addr = SocketAddr::from((args.host, args.port));
    info!("Listening for requests on {}", addr);
    warp::serve(index).try_bind(addr).await;
}
