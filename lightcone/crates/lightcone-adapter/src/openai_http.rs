use crate::{Message, RenderedRequest, Role, Transport, TransportResponse};
use serde_json::{Value, json};
use std::io::{Read, Write};
use std::net::{Shutdown, TcpStream, ToSocketAddrs};
use std::time::Duration;

#[derive(Clone, Debug)]
pub struct OpenAiHttpTransport {
    host: String,
    port: u16,
    base_path: String,
    model: String,
    timeout: Duration,
}

impl OpenAiHttpTransport {
    pub fn new(base_url: &str, model: impl Into<String>) -> Result<Self, String> {
        let without_scheme = base_url
            .strip_prefix("http://")
            .ok_or_else(|| "local OpenAI transport requires an http:// URL".to_string())?;
        let (authority, path) = without_scheme
            .split_once('/')
            .map_or((without_scheme, ""), |(authority, path)| (authority, path));
        let (host, port) = parse_authority(authority)?;
        if !matches!(host.as_str(), "127.0.0.1" | "localhost" | "::1") {
            return Err("OpenAI transport is restricted to a loopback local-LLM endpoint".into());
        }
        let model = model.into();
        if model.trim().is_empty() {
            return Err("model identity must not be empty".into());
        }
        Ok(Self {
            host,
            port,
            base_path: format!("/{}", path.trim_matches('/')),
            model,
            timeout: Duration::from_secs(1800),
        })
    }

    pub fn with_timeout(mut self, timeout: Duration) -> Result<Self, String> {
        if timeout.is_zero() {
            return Err("transport timeout must be positive".into());
        }
        self.timeout = timeout;
        Ok(self)
    }

    fn endpoint(&self) -> String {
        let base = self.base_path.trim_end_matches('/');
        if base.ends_with("/chat/completions") {
            base.to_string()
        } else if base.is_empty() {
            "/v1/chat/completions".into()
        } else {
            format!("{base}/chat/completions")
        }
    }
}

impl Transport for OpenAiHttpTransport {
    fn complete(&mut self, request: &RenderedRequest) -> Result<TransportResponse, String> {
        let body = json!({
            "model": self.model,
            "messages": request.messages.iter().map(message_json).collect::<Vec<_>>(),
            "tools": request.tools,
            "stream": false,
            "max_tokens": request.max_output_tokens,
            "temperature": if request.deterministic { 0.0 } else { 0.8 },
            "seed": request.seed,
        });
        let encoded = serde_json::to_vec(&body).map_err(|error| error.to_string())?;
        let address = format!("{}:{}", self.host, self.port);
        let socket = address
            .to_socket_addrs()
            .map_err(|error| format!("cannot resolve local model endpoint: {error}"))?
            .next()
            .ok_or_else(|| "local model endpoint resolved to no address".to_string())?;
        let mut stream = TcpStream::connect_timeout(&socket, self.timeout)
            .map_err(|error| format!("cannot connect to local model: {error}"))?;
        stream
            .set_read_timeout(Some(self.timeout))
            .and_then(|()| stream.set_write_timeout(Some(self.timeout)))
            .map_err(|error| format!("cannot configure local model socket: {error}"))?;
        let request_head = format!(
            "POST {} HTTP/1.1\r\nHost: {}:{}\r\nContent-Type: application/json\r\nAccept: application/json\r\nConnection: close\r\nContent-Length: {}\r\n\r\n",
            self.endpoint(),
            self.host,
            self.port,
            encoded.len()
        );
        stream
            .write_all(request_head.as_bytes())
            .and_then(|()| stream.write_all(&encoded))
            .and_then(|()| stream.flush())
            .map_err(|error| format!("cannot write local model request: {error}"))?;
        stream
            .shutdown(Shutdown::Write)
            .map_err(|error| format!("cannot finish local model request: {error}"))?;
        let mut response = Vec::new();
        stream
            .read_to_end(&mut response)
            .map_err(|error| format!("cannot read local model response: {error}"))?;
        parse_response(&response, &self.model)
    }
}

fn parse_authority(authority: &str) -> Result<(String, u16), String> {
    if let Some(rest) = authority.strip_prefix('[') {
        let (host, port) = rest
            .split_once("]: ")
            .or_else(|| rest.split_once("]:"))
            .ok_or_else(|| "bracketed IPv6 endpoint requires a port".to_string())?;
        return Ok((host.to_string(), parse_port(port)?));
    }
    let (host, port) = authority
        .rsplit_once(':')
        .ok_or_else(|| "local model endpoint requires an explicit port".to_string())?;
    if host.is_empty() {
        return Err("local model endpoint host is empty".into());
    }
    Ok((host.to_string(), parse_port(port)?))
}

fn parse_port(value: &str) -> Result<u16, String> {
    value
        .parse::<u16>()
        .ok()
        .filter(|port| *port > 0)
        .ok_or_else(|| "local model endpoint has an invalid port".to_string())
}

fn message_json(message: &Message) -> Value {
    let role = match message.role {
        Role::System => "system",
        Role::User => "user",
        Role::Assistant => "assistant",
        Role::Tool => "tool",
    };
    let mut value = json!({"role": role, "content": message.content});
    if let Some(name) = &message.name {
        value["name"] = Value::String(name.clone());
    }
    if let Some(tool_call_id) = &message.tool_call_id {
        value["tool_call_id"] = Value::String(tool_call_id.clone());
    }
    value
}

fn parse_response(response: &[u8], requested_model: &str) -> Result<TransportResponse, String> {
    let boundary = response
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .ok_or_else(|| "local model returned a malformed HTTP response".to_string())?;
    let header = std::str::from_utf8(&response[..boundary])
        .map_err(|_| "local model returned non-UTF-8 HTTP headers".to_string())?;
    let status = header
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|value| value.parse::<u16>().ok())
        .ok_or_else(|| "local model returned no HTTP status".to_string())?;
    let raw_body = &response[boundary + 4..];
    let body = if header.lines().any(|line| {
        line.to_ascii_lowercase()
            .starts_with("transfer-encoding: chunked")
    }) {
        decode_chunked(raw_body)?
    } else {
        raw_body.to_vec()
    };
    if !(200..300).contains(&status) {
        return Err(format!(
            "local model returned HTTP {status}: {}",
            String::from_utf8_lossy(&body)
        ));
    }
    let value: Value = serde_json::from_slice(&body)
        .map_err(|error| format!("local model returned invalid JSON: {error}"))?;
    let message = value
        .pointer("/choices/0/message")
        .ok_or_else(|| "local model response has no first message".to_string())?;
    let visible_text = match message.get("content") {
        Some(Value::String(text)) => text.clone(),
        Some(Value::Null) | None => String::new(),
        Some(other) => serde_json::to_string(other).map_err(|error| error.to_string())?,
    };
    let tool_calls = message
        .get("tool_calls")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    Ok(TransportResponse {
        visible_text,
        tool_calls,
        prompt_tokens: value
            .pointer("/usage/prompt_tokens")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok()),
        completion_tokens: value
            .pointer("/usage/completion_tokens")
            .and_then(Value::as_u64)
            .and_then(|value| usize::try_from(value).ok()),
        stop_reason: value
            .pointer("/choices/0/finish_reason")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string(),
        model_identity: value
            .get("model")
            .and_then(Value::as_str)
            .unwrap_or(requested_model)
            .to_string(),
        runtime_identity: "openai-compatible-local-http".into(),
    })
}

fn decode_chunked(body: &[u8]) -> Result<Vec<u8>, String> {
    let mut cursor = 0usize;
    let mut output = Vec::new();
    loop {
        let line_end = body[cursor..]
            .windows(2)
            .position(|window| window == b"\r\n")
            .map(|offset| cursor + offset)
            .ok_or_else(|| "malformed chunked response size".to_string())?;
        let size_text = std::str::from_utf8(&body[cursor..line_end])
            .map_err(|_| "non-UTF-8 chunk size".to_string())?
            .split(';')
            .next()
            .unwrap_or_default();
        let size = usize::from_str_radix(size_text.trim(), 16)
            .map_err(|_| "invalid chunk size".to_string())?;
        cursor = line_end + 2;
        if size == 0 {
            return Ok(output);
        }
        let end = cursor
            .checked_add(size)
            .ok_or_else(|| "chunk size overflow".to_string())?;
        if end + 2 > body.len() || &body[end..end + 2] != b"\r\n" {
            return Err("truncated chunked response".into());
        }
        output.extend_from_slice(&body[cursor..end]);
        cursor = end + 2;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        AdapterCapabilities, AdapterProfile, BoundaryMode, CanonicalRequest, LocalModelAdapter,
        Message, PromptFormat, Role,
    };
    use std::net::TcpListener;
    use std::thread;

    fn profile() -> AdapterProfile {
        AdapterProfile {
            schema_version: 1,
            adapter_id: "openai-fixture".into(),
            runtime_family: "fixture".into(),
            transport: "http_openai".into(),
            prompt_format: PromptFormat::Plain,
            boundary_mode: BoundaryMode::HostInputOutput,
            model_match: json!({}),
            capabilities: AdapterCapabilities {
                raw_logits: false,
                custom_sampler: false,
                grammar: false,
                tools: false,
                streaming: false,
                multimodal: false,
                deterministic: true,
                seed_control: true,
                embeddings: false,
            },
            context_limit: 4096,
            output_limit: 1024,
            conformance_status: "declared".into(),
            conformance_receipt: None,
        }
    }

    #[test]
    fn real_loopback_http_boundary_preserves_roles_and_identity() {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = Vec::new();
            stream.read_to_end(&mut request).unwrap();
            let text = String::from_utf8(request).unwrap();
            assert!(text.starts_with("POST /v1/chat/completions HTTP/1.1"));
            assert!(text.contains(r#""role":"user""#));
            assert!(text.contains(r#""content":"hello""#));
            let body = r#"{"model":"fixture-model","choices":[{"message":{"role":"assistant","content":"world","tool_calls":[]},"finish_reason":"stop"}],"usage":{"prompt_tokens":3,"completion_tokens":1}}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });
        let transport =
            OpenAiHttpTransport::new(&format!("http://{address}/v1"), "requested").unwrap();
        let mut adapter = LocalModelAdapter::new(profile(), transport).unwrap();
        let response = adapter
            .generate(&CanonicalRequest {
                messages: vec![Message {
                    role: Role::User,
                    content: "hello".into(),
                    name: None,
                    tool_call_id: None,
                }],
                boundary: None,
                tools: None,
                max_output_tokens: 16,
                deterministic: true,
                seed: Some(7),
            })
            .unwrap();
        server.join().unwrap();
        assert_eq!(response.visible_text, "world");
        assert_eq!(response.model_identity, "fixture-model");
        assert_eq!(response.prompt_tokens, Some(3));
    }

    #[test]
    fn non_loopback_endpoint_is_rejected() {
        assert!(OpenAiHttpTransport::new("http://192.0.2.1:8000/v1", "model").is_err());
    }
}
