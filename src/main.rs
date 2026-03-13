mod entity;
mod repository;

macro_rules! get_env {
    ($keyword:expr) => {{
        tracing::info!("config: {}", &$keyword);
        std::env::var(&$keyword).expect(&format!("faied while loading env var: {}", &$keyword))
    }};
}

#[tokio::main]
async fn main() {}
