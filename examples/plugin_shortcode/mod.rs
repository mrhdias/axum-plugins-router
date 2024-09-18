//
// Tera Plugin Shortcode implementation
//

use std::collections::HashMap;
use serde::Deserialize;
use tera::Function;

#[derive(Deserialize, Debug)]
pub struct PluginShortcode {}

impl PluginShortcode {
    pub fn new() -> Self {
        PluginShortcode {}
    }
}

impl Function for PluginShortcode {

    fn call(&self,
        args: &HashMap<String, tera::Value>,
    ) -> tera::Result<tera::Value> {
        // Extract arguments

        let route = match args.get("route") {
            Some(value) => value
                .as_str()
                .unwrap()
                .trim_matches(|c| c == '"' || c == '\''),
            None => return Ok(tera::Value::String("no route specified".to_string())),
        };
        let method = match args.get("method") {
            Some(value) => value
                .as_str()
                .unwrap()
                .trim_matches(|c| c == '"' || c == '\''),
            None => "get",
        };
        let data = match args.get("data") {
            Some(value) => value
                .as_str()
                .unwrap()
                .trim_matches(|c| c == '"' || c == '\''),
            None => "",
        };
        let block: bool = match args.get("block") {
            Some(value) => value
                .as_str()
                .unwrap()
                .trim_matches(|c| c == '"' || c == '\'').parse().unwrap(),
            None => false,
        };

        let alt: Option<&str> = args.get("alt").map(|value| 
            value.as_str().unwrap().trim_matches(|c| c == '"' || c == '\'')
        );

        let fragment = if block {
            fetch_shortcode(route, Some(method), Some(data))
        } else {
            fetch_shortcode_js(route, Some(method), Some(data), alt)
        };

        Ok(tera::Value::String(fragment))
    }
}


fn fetch_shortcode_js(
    url: &str,
    method: Option<&str>,
    json_body: Option<&str>,
    alt: Option<&str>,
) -> String {

    let method = method.unwrap_or("GET");
    let json_body = json_body.unwrap_or("{}");

    let fetch_js = match method.to_lowercase().as_str() {
        "get" => format!(r#"const response = await fetch("{}");"#, url),
        "post" => format!(r#"
const request = new Request("{}", {{
    headers: (() => {{
        const headers = new Headers();
        headers.append("Content-Type", "application/json");
        return headers;
    }})(),
    method: "POST",
    body: JSON.stringify({}),
}});
const response = await fetch(request);"#, url, json_body),
        _ => return format!(r#"<output style="background-color:#F44336;color:#fff;padding:6px;">
Invalid method {} for url {} (only GET and POST methods available)
</output>"#, method, url),
    };

    let js_code = format!(r#"<script>
(function () {{
    async function fetchShortcodeData() {{
        try {{
            {}
            if (!response.ok) {{
                throw new Error(`HTTP error! Status: ${{response.status}}`);
            }}
            return await response.text();
        }} catch (error) {{
            console.error("Fetch failed:", error);
            return "";
        }}
    }}
    (async () => {{
        const currentScript = document.currentScript;
        const content = await fetchShortcodeData();
        // console.log(content);
        currentScript.insertAdjacentHTML('beforebegin', content);
        currentScript.remove();
    }})();
}})();
</script>"#,
    fetch_js);

    if method.to_lowercase().as_str() == "get" && alt.is_some() && !alt.unwrap().is_empty() {
        let alt = alt.unwrap();
        js_code.to_string() + &format!(r#"<noscript><a href="{}">{}</a></noscript>"#, url, alt)
    } else {
        js_code
    }
}

pub fn fetch_shortcode(
    url: &str,
    method: Option<&str>,
    json_body: Option<&str>,
) -> String {

    let method = method.unwrap_or("GET");
    let json_body = json_body.unwrap_or("{}");

    let client = reqwest::Client::new();

    let data_to_route = async {
        let response = match method.to_lowercase().as_str() {
            "get" => client.get(url)
                .send()
                .await,
            "post" => client.post(url)
                .header("Content-Type", "application/json")
                .body(json_body.to_owned())
                .send()
                .await,
                _ => return format!("Invalid method: {}", method),
            };

        match response {
            Ok(res) => {
                if res.status().is_success() {
                    res.text().await.unwrap_or_else(|_| "Failed to read response body".into())
                } else {
                    format!("Request failed with status: {}", res.status())
                }
            }
            Err(e) => format!("Request error: {}", e),
        }
    };

    // Use `block_in_place` to run the async function
    // within the blocking context
    tokio::task::block_in_place(||
        // We need to access the current runtime to
        // run the async function
        tokio::runtime::Handle::current()
            .block_on(data_to_route)
    )
}