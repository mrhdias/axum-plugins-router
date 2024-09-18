# axum-router-plugin
Axum Router Plugin - Dynamically loadable libraries and routes

<ins>Attention</ins>: This project is currently in an experimental stage and may contain bugs or limitations.

## Description
Early-stage, experimental Rust project that allows developers to dynamically load and unload shared libraries, similar to enabling or disabling plugins in WordPress. This flexibility enables developers to extend Axum web applications without recompiling the entire application. The system automatically generates routes for library functions and supports integration with template engines like Tera. A simple configuration file manages the loaded libraries, providing flexibility and extensibility for building custom web applications.

Plugins.toml
```toml
[plugins]
plugin_name = { version = "0.1.0", path = "./path/plugins/libplugin_test.so", enabled = true }
```

Shared libraries must implement a `routes` function that returns a JSON structure defining the library's available routes.
```json
{
  "routes": [
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
  ],
  "message": "Success",
  "status": 0
}
```