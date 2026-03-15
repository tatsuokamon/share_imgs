mod engine;
mod entity;
mod repository;
mod ws;

macro_rules! get_env {
    ($keyword:expr) => {{
        tracing::info!("config: {}", &$keyword);
        std::env::var(&$keyword).expect(&format!("faied while loading env var: {}", &$keyword))
    }};
}

macro_rules! get_env_with_parsing {
    ($keyword:expr, $to:ty) => {{
        tracing::info!("config: {}", &$keyword);
        std::env::var(&$keyword)
            .expect(&format!("faied while loading env var: {}", &$keyword))
            .parse::<$to>()
            .expect(&format!("failed while parsing env var: {}", &keyword))
    }};
}

#[tokio::main]
async fn main() {}
