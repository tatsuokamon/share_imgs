mod cloud_flare_filer;
mod db_executor;
mod engine;
mod file_saver;
mod shared_architect;
mod entity;

macro_rules! get_env {
    ($keyword:expr) => {{
        tracing::info!("config: {}", &$keyword);
        std::env::var(&$keyword).expect(&format!("faied while loading env var: {}", &$keyword))
    }};
}

#[tokio::main]
async fn main() {
}
