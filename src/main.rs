//! Default Compute@Edge template program.
use config::{Config, FileFormat};
use fastly::dictionary::Dictionary;
use fastly::http::{HeaderMap, HeaderValue, Method, StatusCode, Uri};
use fastly::{Body, Error, Request, RequestExt, Response, ResponseExt};
use jwt_simple::claims::JWTClaims;
use jwt_simple::prelude::{RS256PublicKey, RSAPublicKeyLike, Token};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str;
use url::{ParseError, Url};

/// The name of a backend server associated with this service.
///
const BACKEND_NAME: &str = "ShastaRain_backend";
const LOGGING_ENDPOINT: &str = "SceneSwitch_syslog";
const DICT_NAME: &str = "cut_scenes";

/// The entry point for your application.
///
/// This function is triggered when your service receives a client request. It could be used to
/// route based on the request properties (such as method or path), send the request to a backend,
/// make completely new requests, and/or generate synthetic responses.
///
/// If `main` returns an error, a 500 error response will be delivered to the client.
#[fastly::main]
fn main(mut req: Request<Body>) -> Result<impl ResponseExt, Error> {
    logging_init();

    match (req.method(), req.uri().path()) {
        (&Method::GET, path) if path.ends_with(".ts") => {
            let segment_num = get_segment_num(path).unwrap_or(0);
            if is_segment_cut_scene(segment_num) {
                let client_ip = fastly::downstream_client_ip_addr().unwrap();
                let geo = fastly::geo::geo_lookup(client_ip).unwrap();
                if geo.country_code() == "US" {
                    log::debug!("We're in the USA!");
                } else {
                    log::debug!("We're in: {}", geo.country_code());
                }
                /*
                let mut url = Url::parse(req.uri().to_string().as_ref()).unwrap();
                url = url
                    .join(format!("b.segment_{}.ts", segment_num).as_ref())
                    .unwrap();
                let new_uri = url.to_string().parse::<Uri>().unwrap();
                *req.uri_mut() = new_uri;
                 */
                Ok(req.send(BACKEND_NAME)?)
            } else {
                // Not a cut scense so just return.
                Ok(req.send(BACKEND_NAME)?)
            }
        }
        // The file needs no special processing so just send it through.
        _ => Ok(req.send(BACKEND_NAME)?),
    }
}

fn is_segment_cut_scene(seg_num: u32) -> bool {
    let scene_dict = Dictionary::open(DICT_NAME);
    let scene_list: Vec<u32> = scene_dict
        .get("scenes")
        .unwrap_or("Auth Cookie not found".to_string())
        .split(',')
        .map(|x| x.parse::<u32>().unwrap())
        .collect();
    scene_list.contains(&seg_num)
}

fn get_segment_num(input: &str) -> Option<u32> {
    lazy_static! {
        static ref RE: Regex = Regex::new(r"^.*/.*_(?P<seg_num>\d+)\.ts$").unwrap();
    }
    RE.captures(input).and_then(|cap| {
        cap.name("seg_num")
            .map(|seg_num| seg_num.as_str().parse::<u32>().unwrap())
    })
}

/// This function reads the fastly.toml file and gets the deployed version. This is only run at
/// compile time. Since we bump the version number after building (during the deploy) we return
/// the version incremented by one so the version returned will match the deployed version.
fn get_version() -> i32 {
    Config::new()
        .merge(config::File::from_str(
            include_str!("../fastly.toml"), // assumes the existence of fastly.toml
            FileFormat::Toml,
        ))
        .unwrap()
        .get_str("version")
        .unwrap()
        .parse::<i32>()
        .unwrap_or(0)
        + 1
}

// Boiler plate function that I will include in every app until we have something in place that
// doe this.
fn logging_init() {
    log_fastly::Logger::builder()
        .max_level(log::LevelFilter::Debug)
        .default_endpoint(LOGGING_ENDPOINT)
        .init();

    fastly::log::set_panic_endpoint(LOGGING_ENDPOINT).unwrap();

    log::debug!("Dynamic Scene Switching Demo Version:{}", get_version());
}
