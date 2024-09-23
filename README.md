# axum-router-plugin
Axum Router Plugin - Dynamically loadable libraries and routes

<ins>Attention</ins>: This project is in an experimental stage and may contain bugs or limitations. It has only been tested in a Linux environment.

## Description
Early-stage, experimental Rust project that allows developers to dynamically load and unload shared libraries, similar to enabling or disabling plugins in WordPress. This flexibility enables developers to extend Axum web applications without recompiling the entire application. The system automatically generates routes for library functions and supports integration with template engines like Tera. A simple configuration file manages the loaded libraries, providing flexibility and extensibility for building custom web applications.

## Usage Example:
```rust
use axum_router_plugin::Plugins;
use axum::{
  routing::get,
  Router,
};

#[tokio::main]
async fn main() {
  // Load plugins from the Plugins.toml file.
  // You can change the location of the Plugins.toml file by setting
  // the environment variable PLUGINS_CONF, for example:
  // export PLUGINS_CONF=plugins/Plugins.toml
  //
  // Set the argument to true if you want to add the plugin name to the routes.
  let axum_plugins = Plugins::new(Some(true));

  // Load the plugins and create a router with the loaded plugins.
  // If loading fails, the program will panic with an error message.
  let plugins_router = match axum_plugins.load() {
    Ok(router) => router,
    Err(err) => panic!("Error loading plugins: {}", err),
  };

  // Build our application with a route.
  // The plugins are nested under the "/plugin" path.
  let _app = Router::new()
    .route("/", get(|| async {
      "Hello world!"
    }))
    .nest("/plugin", plugins_router);
}
```

## Plugin Configuration:
To load the shared libraries, there must be a `Plugins.toml` file containing a list of libraries and their status. This file specifies the path, version, and whether each plugin is enabled.

Example `Plugins.toml` entry:
```toml
[plugin_name]
path = "path/to/plugin.so"
version = "1.0"
enabled = true
```
Is possible change the location of the Plugins.toml file by setting the environment variable PLUGINS_CONF.

Example:
```sh
export PLUGINS_CONF=plugins/Plugins.toml
# unset values in bash
unset PLUGINS_CONF
# unset values in fish
set --erase PLUGINS_CONF
```

## How to test the provided example:
```sh
git clone https://github.com/mrhdias/axum-router-plugin
cd axum-router-plugin
ls -la plugins
nano -w Plugins.toml
cargo run --example app
```
In the `examples` directory, there is a `templates` directory that demonstrates how plugin routes can be used with shortcodes to display content provided by plugins. The shortcodes are available through the Tera template engine.

Usage Example:
```html
{{ plugin(route="/plugin/foo-bar/test-get", method='get', jscaller="true") | safe }}

{% set my_vegetables = '["carrot", "potato", "tomato", "beet"]' %}
{% set my_bag = '{
    "fruits": ["apple", "orange", "banana"],
    "vegetables": ' ~ my_vegetables ~ '
}' %}
Data from plugin function: 
<pre>
{{ plugin(route="/plugin/foo-bar/test-json", method='post', data=my_bag, jscaller="true") | safe }}
</pre>
```

## Plugin Examples

For more information about the plugins, refer to the plugin skeleton:
```sh
git clone https://github.com/mrhdias/arp-skeleton
cd arp-skeleton
cargo build --release
cp target/release/libarp_skeleton.so ../axum-router-plugin/plugins
```
Another plugin example:
```sh
git clone https://github.com/mrhdias/arp-foo-bar
cd arp-foo-bar
cargo build --release
cp target/release/libarp_foo_bar.so ../axum-router-plugin/plugins
```

Shared libraries must implement a `routes` function that returns a JSON array containing all available routes for the library.

Example JSON:
```json
[
  {
    "path": "/test-get",
    "function": "test_get",
    "method_router": "get",
    "response_type": "html"
  },
  {
    "path": "/test-post",
    "function": "test_post",
    "method_router": "post",
    "response_type": "html"
  },
  {
    "path": "/test-json",
    "function": "test_json",
    "method_router": "post",
    "response_type": "json"
  },
  {
    "path": "/version",
    "function": "version",
    "method_router": "get",
    "response_type": "text"
  }
]
```