use std::net::SocketAddr;

use axum::{extract::Request, http::StatusCode, middleware::Next, response::Response};

use crate::engine::EngineState;

fn generate_rate_limit_tag(ip: &String) -> String {
    format!("rate-limit:ip:{}", ip)
}

pub async fn rate_limit_middleware(
    mut req: Request<axum::body::Body>,
    next: Next,
) -> Result<Response, StatusCode> {
    let state = req
        .extensions()
        .get::<EngineState>()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?
        .clone();

    let ip = if let Some(addr) = req.extensions().get::<SocketAddr>() {
        addr.ip().to_string()
    } else {
        return Err(StatusCode::BAD_REQUEST);
    };

    let key = generate_rate_limit_tag(&ip);
    let mut conn = state
        .pool
        .get()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let count: usize = redis::cmd("INCR")
        .arg(&key)
        .query_async(&mut *conn)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    if count == 1 {
        let _: () = redis::cmd("EXPIRE")
            .arg(&key)
            .arg(60)
            .query_async(&mut *conn)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    }

    if count > state.req_per_minute {
        return Err(StatusCode::TOO_MANY_REQUESTS);
    }

    Ok(next.run(req).await)
}
