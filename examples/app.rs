//
// Usage example of axum router plugin
//

mod plugin_shortcode;

use axum_router_plugin;
use axum::{
    extract::{Extension, Request},
    response::Html,
    routing::get,
    Router,
    ServiceExt
};
use tower_http::normalize_path::NormalizePathLayer;
use tower::Layer;
use tera::{Context, Tera};

const ADDRESS: &str = "127.0.0.1:8080";

async fn test(
    Extension(tera): Extension<Tera>,
) -> Html<String> {

    let context = Context::new();
    // Render the template with the context
    let rendered = tera
        .render("plugin_test_shortcodes.html", &context)
        .unwrap();

    Html(rendered)
}


#[tokio::main]
async fn main() {

    // Load plugins from the Plugins.toml file
    let axum_plugins = axum_router_plugin::Plugins::new(Some(true));
    let plugins_router = match axum_plugins.load() {
        Ok(router) => router,
        Err(err) => panic!("Error loading plugins: {}", err),
    };

    let mut tera = Tera::new("examples/templates/**/*").unwrap();

    let plugin_shortcode = plugin_shortcode::PluginShortcode::new();

    // Register the custom function
    tera.register_function("plugin", plugin_shortcode);

    // Build our application with a route
    let app = Router::new()
        .route("/", get(|| async {
            "Hello world!"
        }))
        .route("/test", get(test))
        .nest("/plugin", plugins_router)
        .layer(Extension(tera));

    let app = NormalizePathLayer::trim_trailing_slash()
        .layer(app);

    // Run the server
    let listener = tokio::net::TcpListener::bind(ADDRESS)
        .await
        .unwrap();

    let url = format!("http://{}/test", ADDRESS);
    if let Err(e) = open::that(&url) {
        eprintln!("Failed to open URL: {}", e);
    }
    println!("Point your browser to this url: {} if not opened automatically", url);

    axum::serve(listener, ServiceExt::<Request>::into_make_service(app))
        .await
        .unwrap();
}