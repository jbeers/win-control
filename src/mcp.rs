use rmcp::{
    tool,
    tool_router,
    handler::server::router::{tool::ToolRouter, Router},
    handler::server::wrapper::{Json, Parameters},
    service::serve_server,
};
use serde::{Deserialize, Serialize};

use schemars::JsonSchema;

#[derive(Serialize, Deserialize, Debug, Clone, JsonSchema)]
#[schemars(crate = "rmcp::schemars")]
pub struct ToolRequest {
    pub tool: String,
    pub option: String,
}

#[derive(Clone)]
pub struct AudioTools {
    tool_router: ToolRouter<Self>,
}




#[tool_router]
impl AudioTools {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(name = "change_audio_device", description = "Change the default audio output device.")]
    async fn change_audio_device(&self, params: Parameters<ToolRequest>) -> Result<Json<serde_json::Value>, String> {
        match params.0.option.as_str() {
            "headphones" => {
                #[cfg(target_os = "windows")]
                crate::audio::set_default_output_by_name("headphone");
                Ok(Json(serde_json::json!({"result": "ok"})))
            },
            "usb speaker" => {
                #[cfg(target_os = "windows")]
                crate::audio::set_default_output_by_name("usb speaker");
                Ok(Json(serde_json::json!({"result": "ok"})))
            },
            _ => Ok(Json(serde_json::json!({"error": "invalid tool or option"}))),
        }
    }
}

// Implement the server trait (all methods have default impls)
impl rmcp::handler::server::ServerHandler for AudioTools {}

pub fn start_mcp_server() {
    std::thread::spawn(|| {
        let rt = tokio::runtime::Runtime::new().expect("Failed to create Tokio runtime");
        rt.block_on(async {
            use tokio::net::TcpListener;

            let listener = TcpListener::bind("127.0.0.1:30331")
                .await
                .expect("Failed to bind MCP TCP port");
            loop {
                match listener.accept().await {
                    Ok((stream, _addr)) => {
                        let service = Router::new(AudioTools::new());
                        tokio::spawn(async move {
                            let _ = serve_server(service, stream).await;
                        });
                    }
                    Err(e) => {
                        eprintln!("MCP accept error: {e}");
                        break;
                    }
                }
            }
        });
    });
}
