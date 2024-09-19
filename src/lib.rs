//
// plugins module
//

use std::path::PathBuf;
use serde::Deserialize;
use serde_json::Value;
use std::collections::HashMap;
use axum::{
    extract::RawQuery,
    response::{Html, Json, IntoResponse},
    routing::{get, post},
    Router,
};
use hyper::{HeaderMap, header::HeaderValue};
use libloading::{Library, Symbol};
use std::ffi::{c_char, CStr, CString};
use once_cell::sync::Lazy;
use std::sync::Mutex;

#[derive(Debug, Deserialize)]
struct PluginRoute {
    path: String,
    function: String,
    method_router: String,
    response_type: String,
}

#[derive(Debug, Deserialize)]
struct RoutesResponse {
    routes: Vec<PluginRoute>,
    message: String,
    status: i32, 
}

#[derive(Debug, Clone, Deserialize)]
struct Plugin {
    version: String,
    path: String,
    enabled: bool,
    // key: Option<String>,
}

#[derive(Deserialize, Debug)]
pub struct PluginsConf {
    plugins: HashMap<String, Plugin>,
}

#[derive(Deserialize, Debug)]
pub struct Plugins {
    name_to_route: bool,
}

static DEBUG: Lazy<bool> = Lazy::new(|| {
    std::env::var("DEBUG")
        .map(|val| val == "true")
        .unwrap_or(false)
});

static LIBRARY_FILE: &str = "Plugins.toml";

static LIBRARIES: Lazy<HashMap<String, Mutex<Library>>> = Lazy::new(|| {
    let toml_content = match std::fs::read_to_string(LIBRARY_FILE) {
        Ok(content) => content,
        Err(e) => panic!("Error reading Plugins.toml: {}", e),
    };

    // Parse the TOML content into the PluginsConfig struct
    let plugins_config: PluginsConf = match toml::from_str(&toml_content) {
        Ok(config) => config,
        Err(e) => panic!("Error parsing Plugins.toml: {}", e),
    };
    
    let mut libraries = HashMap::new();

    // Load each library
    for (name, plugin) in plugins_config.plugins {
        let plugin_path = PathBuf::from(&plugin.path);

        // Skip disabled plugins
        if !plugin.enabled {
            eprintln!(
                "Skipping plugin: {}: {} - disabled", 
                name, plugin_path.to_string_lossy()
            );
            continue;
        }

        // Check if plugin file exists
        if !plugin_path.is_file() {
            eprintln!(
                "Skipping plugin: {}: {} - plugin file not found", 
                name, plugin_path.to_string_lossy()
            );
            continue;
        }

        let lib = unsafe {
            match Library::new(&plugin_path) {
                Ok(lib) => lib,
                Err(e) => panic!("Error loading library {}: {}", plugin_path.to_string_lossy(), e),
            }
        };

        /*
        if let Some(key) = plugin.key {
            if key.is_empty() {
                eprintln!(
                    "Skipping plugin: {}: {} - empty key", 
                    name, plugin_path.to_string_lossy()
                );
                continue;
            }

            eprintln!(
                "Plugin: {} - Key: {}", 
                name, key
            );

            let key_fn: Symbol<extern "C" fn(*const c_char) -> *const c_char> = unsafe {
                match lib.get(b"key\0") {
                    Ok(symbol) => symbol,
                    Err(e) => panic!("Error getting key function: {}", e),
                }
            };

            let c_key = CString::new(key).unwrap();

            let result = key_fn(c_key.as_ptr());

            // clean this from memory
            let json_data = unsafe {
                CStr::from_ptr(result).to_string_lossy().into_owned()
            };

            println!("Result: {}", json_data);

            let free_fn: Symbol<extern "C" fn(*mut c_char)> = unsafe {
                match lib.get(b"free\0") {
                    Ok(symbol) => symbol,
                    Err(e) => panic!("Error getting free function: {}", e),
                }
            };
        
            free_fn(result as *mut c_char);
        }
        */

        println!("Plugin: {} Version: {}", name, plugin.version);

        libraries.insert(name, Mutex::new(lib));
    }

    libraries
});

impl Plugins {

    async fn handle_route(
        headers: HeaderMap,
        body: String,
        function: extern "C" fn(*mut HeaderMap, *const c_char) -> *const c_char,
        free: extern "C" fn(*mut c_char),
    ) -> String {

        if *DEBUG { println!("Handle Route Header Map: {:?}", headers); }

        tokio::task::spawn_blocking(move || -> String {
            // Box the headers and convert the body to a CString
            let box_headers = Box::new(headers);
            let c_body = CString::new(body).unwrap();
    
            // Call the external C function with the appropriate pointers
            let ptr = function(Box::into_raw(box_headers), c_body.as_ptr());
            if ptr.is_null() {
                panic!("Received null pointer from function");
            }

            // clean this from memory
            unsafe {
                let data = CStr::from_ptr(ptr).to_string_lossy().into_owned();
                free(ptr as *mut c_char);
                data
            }
        }).await.unwrap()
    }

    fn set_response(
        response: &str,
        response_type: &str,
    ) -> axum::response::Response {

        match response_type.to_lowercase().as_str() {
            "text" => response.to_string()
                .into_response(),
            "html" => Html(response.to_string())
                .into_response(),
            "json" => {
                // println!("Json String Response : {}", response.to_string());
                let v: Value = match serde_json::from_str(response) {
                    Ok(json_value) => json_value,
                    Err(e) => {
                        eprintln!("Error parsing JSON: {}", e);
                        serde_json::Value::String(format!("Error parsing JSON: {}", e))
                    },
                };
                Json(v).into_response()
            },
            _ => panic!("Unsupported response format"),
        }
    }

    pub fn load(&self) -> Result<Router, libloading::Error> {

        let message = || -> String {
            let count = LIBRARIES.len();
            format!("Loaded plugins: {}", count)
        }();

        let mut router: Router = Router::new()
            .route("/", get(|| async {
                message
            })
        );
        
        if LIBRARIES.is_empty() {
            return Ok(router);
        }

        for (name, lib) in LIBRARIES.iter() {

            let lib = match lib.lock() {
                Ok(lib) => lib,
                Err(e) => panic!("Error locking library: {}", e),
            };

            let routes_fn: Symbol<extern "C" fn() -> *const c_char> = unsafe {
                match lib.get(b"routes\0") {
                    Ok(symbol) => symbol,
                    Err(e) =>  panic!("Error getting routes: {}", e),
                }
            };

            let list_ptr = routes_fn();

            if list_ptr.is_null() {
                panic!("Received null pointer from routes function");
            }

            // clean this from memory
            let json_data = unsafe {
                CStr::from_ptr(list_ptr).to_string_lossy().into_owned()
            };

            // Clean up memory allocated by plugin if necessary
            let free_fn: Symbol<extern "C" fn(*mut c_char)> = unsafe {
                match lib.get(b"free\0") {
                    Ok(symbol) => symbol,
                    Err(e) => panic!("Error getting free function: {}", e),
                }
            };
        
            // Free the memory
            free_fn(list_ptr as *mut c_char);

            if *DEBUG { println!("Routes Json: {}", json_data); }

            let routes_response: RoutesResponse = serde_json::from_str(&json_data).unwrap();

            if routes_response.status != 0 {
                panic!("Error loading routes: {}", routes_response.message);
            }

            for route in routes_response.routes {
                // Load the plugin_route_function

                let function: Symbol<extern "C" fn(*mut HeaderMap, *const c_char) -> *const c_char> = unsafe {
                    match lib.get(route.function.as_bytes()) {
                        Ok(symbol) => symbol,
                        Err(e) => panic!("Error getting plugin_route_function: {}", e),
                    }
                };

                // Move the loaded function into the closure to avoid borrowing `lib`
                let cloned_fn = *function;
                let cloned_free_fn = *free_fn;

                // check if route.path start with "/"
                let route_path = if self.name_to_route {
                    format!("/{}/{}", &name, if route.path.starts_with("/") {
                        &route.path[1..]
                    } else {
                        &route.path
                    })
                } else {
                    route.path
                };

                // https://docs.rs/axum/latest/axum/extract/index.html
                let r = Router::new()
                    .route(&route_path, match route.method_router.to_lowercase().as_str() {
                        "get" => get(move |
                            RawQuery(query): RawQuery,
                            mut headers: HeaderMap,
                            body: String,
                        | async move {
                            if let Some(query) = query {
                                headers.insert("x-raw-query", HeaderValue::from_str(&query).unwrap());
                            }
                            let response = Self::handle_route(
                                headers,
                                body, 
                                cloned_fn, 
                                cloned_free_fn,
                            ).await;
                            Self::set_response(&response, &route.response_type)
                        }),
                        "post" => post(move |
                            RawQuery(query): RawQuery,
                            mut headers: HeaderMap,
                            body: String,
                        | async move {
                            if let Some(query) = query {
                                headers.insert("x-raw-query", HeaderValue::from_str(&query).unwrap());
                            }
                            let response = Self::handle_route(
                                headers,
                                body, 
                                cloned_fn, 
                                cloned_free_fn,
                            ).await;
                            Self::set_response(&response, &route.response_type)
                        }),
                        _ => panic!("Unsupported method: {:?}", route.method_router),
                    }
                );
                router = router.merge(r);
            }
        }

        Ok(router)
    }

    pub fn new(
        name_to_route: Option<bool>,
    ) -> Self {

        Plugins {
            name_to_route: match name_to_route {
                Some(true) => true,
                Some(false) => false,
                None => false,
            },
        }
    }
}