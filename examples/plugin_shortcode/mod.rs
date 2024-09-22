//
// Tera Plugin Shortcode implementation
//

use std::collections::HashMap;
use serde::Deserialize;
use tera::Function;
use once_cell::sync::Lazy;

static CLIENT: Lazy<reqwest::Client> = Lazy::new(|| reqwest::Client::new());

use crate::ADDRESS;

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

        let js_caller = match args.get("jscaller") {
            Some(value) => value
                .as_str()
                .unwrap()
                .trim_matches(|c| c == '"' || c == '\'')
                .parse()
                .unwrap_or(false),
            None => false,
        };

        let alt: Option<&str> = args.get("alt").map(|value| 
            value.as_str().unwrap().trim_matches(|c| c == '"' || c == '\'')
        );

        let fragment = if js_caller {
            fetch_shortcode_js(route, Some(method), Some(data), alt)
        } else {
            fetch_shortcode(route, Some(method), Some(data))
        };

        Ok(tera::Value::String(fragment))
    }
}

pub fn fetch_shortcode_js(
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
        _ => return format!(r#"<output style="background-color:#f44336;color:#fff;padding:6px;">
Invalid method {} for url {} (only GET and POST methods available)
</output>"#, method, url),
    };

    // reScript function ia a trick to make the Javascript code work when inserted.
    // Replace it with another clone element script.
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
    function reScript(helper) {{
        for (const node of helper.childNodes) {{
            if (node.hasChildNodes()) {{
                reScript(node);
            }}
            if (node.nodeName === 'SCRIPT') {{
                const script = document.createElement('script');
                script.type = "text/javascript";
                script.textContent = node.textContent;
                node.replaceWith(script);
            }}
        }}
    }}
    (async () => {{
        const currentScript = document.currentScript;
        const content = await fetchShortcodeData();
        // console.log(content);
        const helper = document.createElement('div');
        helper.id = 'helper';
        helper.innerHTML = content;
        reScript(helper);
        currentScript.after(...helper.childNodes);
        currentScript.remove();
    }})();
}})();
</script>"#,
    fetch_js);

    if method.to_lowercase().as_str() == "get" && alt.is_some() {
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

    let url = format!("http://{}{}", ADDRESS, url);

    let data_to_route = async {
        let response = match method.to_lowercase().as_str() {
            "get" => CLIENT.get(url)
                .send()
                .await,
            "post" => CLIENT.post(url)
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