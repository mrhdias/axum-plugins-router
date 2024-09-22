use axum_router_plugin::Plugins;
use axum::{
    routing::get,
    Router,
};

#[tokio::main]
async fn main() {
    // Load plugins from the Plugins.toml file
    let axum_plugins = Plugins::new(Some(true));
    let plugins_router = match axum_plugins.load() {
        Ok(router) => router,
        Err(err) => panic!("Error loading plugins: {}", err),
    };

    // Build our application with a route
    let _app = Router::new()
        .route("/", get(|| async {
            "Hello world!"
        }))
        .nest("/plugin", plugins_router);
}