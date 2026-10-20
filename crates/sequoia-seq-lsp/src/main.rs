#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::io::{BufReader, Read, Write};

use sequoia_seq_lsp::{completions_for, diagnostics_for, hover_for, symbols_for};

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--version" || a == "-V") {
        println!("seq-lsp {VERSION}");
        return;
    }
    if let Some(i) = args.iter().position(|a| a == "--check") {
        let path = args.get(i + 1).cloned().unwrap_or_default();
        std::process::exit(cmd_check(&path));
    }
    run_stdio();
}

fn cmd_check(path: &str) -> i32 {
    if path.is_empty() {
        eprintln!("usage: seq-lsp --check FILE.seq");
        return 2;
    }
    let src = match std::fs::read_to_string(path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{path}:1:1: cannot be read: {e}");
            return 1;
        }
    };
    let diags = diagnostics_for(path, &src);
    for d in &diags {
        println!("{}:{}:{}: {}", path, d.line + 1, d.col + 1, d.message);
    }
    if diags.is_empty() { 0 } else { 1 }
}

const MAX_MESSAGE_BYTES: usize = 16 * 1024 * 1024;

const MAX_HEADER_LINE_BYTES: usize = 8 * 1024;

fn read_message<R: std::io::Read>(reader: &mut BufReader<R>) -> Option<serde_json::Value> {
    let mut header = String::new();
    let mut content_length: Option<usize> = None;
    loop {
        header.clear();

        let mut line: Vec<u8> = Vec::new();
        let mut buf = [0u8; 1];
        loop {
            match reader.read(&mut buf) {
                Ok(0) => return None,
                Ok(_) => {
                    line.push(buf[0]);
                    if buf[0] == b'\n' {
                        break;
                    }
                    if line.len() > MAX_HEADER_LINE_BYTES {
                        return None;
                    }
                }
                Err(_) => return None,
            }
        }
        let line = String::from_utf8_lossy(&line).to_string();
        let t = line.trim();
        if t.is_empty() {
            break;
        }
        if let Some(v) = t.strip_prefix("Content-Length:") {
            content_length = v.trim().parse().ok();
        }
    }
    let len = content_length?;
    if len > MAX_MESSAGE_BYTES {
        return None;
    }
    let mut body = vec![0u8; len];
    reader.read_exact(&mut body).ok()?;
    serde_json::from_slice(&body).ok()
}

fn write_message(msg: &serde_json::Value) {
    let body = serde_json::to_string(msg).unwrap_or_else(|_| "{}".to_string());
    let out = std::io::stdout();
    let mut lock = out.lock();
    let _ = write!(lock, "Content-Length: {}\r\n\r\n{body}", body.len());
    let _ = lock.flush();
}

fn resp(id: &serde_json::Value, result: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"jsonrpc":"2.0","id":id,"result":result})
}

fn err_resp(id: &serde_json::Value, code: i32, message: &str) -> serde_json::Value {
    serde_json::json!({"jsonrpc":"2.0","id":id,"error":{"code":code,"message":message}})
}

fn notify(method: &str, params: serde_json::Value) -> serde_json::Value {
    serde_json::json!({"jsonrpc":"2.0","method":method,"params":params})
}

fn uri_to_file(uri: &str) -> String {
    if let Some(path) = uri.strip_prefix("file://") {
        let mut out = String::new();
        let mut chars = path.chars();
        while let Some(c) = chars.next() {
            if c == '%' {
                let h: String = chars.by_ref().take(2).collect();
                if let Ok(b) = u8::from_str_radix(&h, 16) {
                    out.push(b as char);
                    continue;
                }
                out.push('%');
                out.push_str(&h);
            } else {
                out.push(c);
            }
        }

        if out.len() > 3 && out.starts_with('/') && out.chars().nth(2) == Some(':') {
            return out[1..].to_string();
        }
        return out;
    }
    uri.to_string()
}

fn run_stdio() {
    let stdin = std::io::stdin();
    let mut reader = BufReader::new(stdin.lock());
    let mut docs: HashMap<String, String> = HashMap::new();
    let mut shutdown_requested = false;

    while let Some(msg) = read_message(&mut reader) {
        let method = msg.get("method").and_then(|m| m.as_str()).unwrap_or("");
        let id = msg.get("id").cloned().unwrap_or(serde_json::Value::Null);
        let params = msg
            .get("params")
            .cloned()
            .unwrap_or(serde_json::Value::Null);
        match method {
            "initialize" => {
                let result = serde_json::json!({
                    "capabilities": {
                        "textDocumentSync": 1,
                        "completionProvider": {
                            "triggerCharacters": [":", ".", "(", "\"", "{", " "]
                        },
                        "hoverProvider": true,
                        "documentSymbolProvider": true
                    },
                    "serverInfo": {"name": "seq-lsp", "version": VERSION}
                });
                write_message(&resp(&id, result));
            }
            "initialized" => {}
            "shutdown" => {
                shutdown_requested = true;
                write_message(&resp(&id, serde_json::Value::Null));
            }
            "exit" => break,
            "textDocument/didOpen" => {
                if let (Some(uri), Some(text)) = (
                    params.pointer("/textDocument/uri").and_then(|v| v.as_str()),
                    params
                        .pointer("/textDocument/text")
                        .and_then(|v| v.as_str()),
                ) {
                    docs.insert(uri.to_string(), text.to_string());
                    publish(uri, text);
                }
            }
            "textDocument/didChange" => {
                if let Some(uri) = params.pointer("/textDocument/uri").and_then(|v| v.as_str()) {
                    if let Some(text) = params
                        .pointer("/contentChanges/0/text")
                        .and_then(|v| v.as_str())
                    {
                        docs.insert(uri.to_string(), text.to_string());
                        publish(uri, text);
                    } else if let Some(last) = params
                        .pointer("/contentChanges")
                        .and_then(|v| v.as_array())
                        .and_then(|arr| arr.last())
                        .and_then(|v| v.get("text"))
                        .and_then(|v| v.as_str())
                    {
                        docs.insert(uri.to_string(), last.to_string());
                        publish(uri, last);
                    }
                }
            }
            "textDocument/didClose" => {
                if let Some(uri) = params.pointer("/textDocument/uri").and_then(|v| v.as_str()) {
                    docs.remove(uri);
                    write_message(&notify(
                        "textDocument/publishDiagnostics",
                        serde_json::json!({"uri": uri, "diagnostics": []}),
                    ));
                }
            }
            "textDocument/didSave" => {
                if let Some(uri) = params.pointer("/textDocument/uri").and_then(|v| v.as_str()) {
                    if let Some(text) = params.pointer("/text").and_then(|v| v.as_str()) {
                        docs.insert(uri.to_string(), text.to_string());
                        publish(uri, text);
                    } else if let Some(cached) = docs.get(uri).cloned() {
                        publish(uri, &cached);
                    }
                }
            }
            "textDocument/completion" => {
                let items = completion_items(&docs, &params);
                write_message(&resp(
                    &id,
                    serde_json::json!({"isIncomplete": false, "items": items}),
                ));
            }
            "textDocument/hover" => {
                let hover = hover_at(&docs, &params);
                write_message(&resp(&id, hover));
            }
            "textDocument/documentSymbol" => {
                let symbols = symbols_in(&docs, &params);
                write_message(&resp(&id, symbols));
            }
            _ => {
                if !id.is_null() {
                    write_message(&err_resp(&id, -32601, "unsupported method"));
                }
            }
        }
        if shutdown_requested && method == "exit" {
            break;
        }
    }
}

fn publish(uri: &str, text: &str) {
    let file = uri_to_file(uri);
    let diags = diagnostics_for(&file, text);
    let items: Vec<serde_json::Value> = diags
        .iter()
        .map(|d| {
            serde_json::json!({
                "range": {
                    "start": {"line": d.line, "character": d.col},
                    "end": {"line": d.line, "character": d.end_col}
                },
                "severity": 1,
                "source": "seq",
                "message": d.message
            })
        })
        .collect();
    write_message(&notify(
        "textDocument/publishDiagnostics",
        serde_json::json!({"uri": uri, "diagnostics": items}),
    ));
}

fn doc_text(
    docs: &HashMap<String, String>,
    params: &serde_json::Value,
) -> Option<(String, u32, u32)> {
    let uri = params.pointer("/textDocument/uri")?.as_str()?;
    let text = docs.get(uri)?.clone();
    let line = params.pointer("/position/line")?.as_u64()? as u32;
    let character = params.pointer("/position/character")?.as_u64()? as u32;
    Some((text, line, character))
}

fn completion_items(
    docs: &HashMap<String, String>,
    params: &serde_json::Value,
) -> Vec<serde_json::Value> {
    let Some((text, line, character)) = doc_text(docs, params) else {
        return Vec::new();
    };
    completions_for(&text, line, character)
        .into_iter()
        .map(|c| {
            serde_json::json!({
                "label": c.label,
                "kind": c.kind,
                "detail": c.detail
            })
        })
        .collect()
}

fn hover_at(docs: &HashMap<String, String>, params: &serde_json::Value) -> serde_json::Value {
    let Some((text, line, character)) = doc_text(docs, params) else {
        return serde_json::Value::Null;
    };
    match hover_for(&text, line, character) {
        Some(md) => serde_json::json!({"contents": {"kind": "markdown", "value": md}}),
        None => serde_json::Value::Null,
    }
}

fn symbols_in(docs: &HashMap<String, String>, params: &serde_json::Value) -> serde_json::Value {
    let uri = params
        .pointer("/textDocument/uri")
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let text = docs.get(uri).cloned().unwrap_or_default();
    let items: Vec<serde_json::Value> = symbols_for(&text)
        .into_iter()
        .map(|s| {
            let kind = match s.kind.as_str() {
                "app" => 1,
                "component" => 5,
                "window" => 5,
                "style" => 5,
                _ => 22,
            };
            serde_json::json!({
                "name": s.name,
                "kind": kind,
                "location": {
                    "uri": uri,
                    "range": {
                        "start": {"line": s.line, "character": 0},
                        "end": {"line": s.line, "character": 0}
                    }
                }
            })
        })
        .collect();
    serde_json::Value::Array(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn reader(bytes: &[u8]) -> BufReader<Cursor<Vec<u8>>> {
        BufReader::new(Cursor::new(bytes.to_vec()))
    }

    #[test]
    fn valid_frame_parses() {
        let body = r#"{"jsonrpc":"2.0","id":1,"method":"initialize"}"#;
        let frame = format!("Content-Length: {}\r\n\r\n{body}", body.len());
        let v = read_message(&mut reader(frame.as_bytes())).expect("ramka");
        assert_eq!(v["method"], "initialize");
    }

    #[test]
    fn oversized_content_length_is_refused() {
        let msg = b"Content-Length: 10000000000000\r\n\r\n";
        assert!(
            read_message(&mut reader(msg)).is_none(),
            "gigantyczna ramka odrzucona bez alokacji"
        );
    }

    #[test]
    fn endless_header_line_is_refused() {
        let msg = vec![b'x'; MAX_HEADER_LINE_BYTES * 4];
        assert!(read_message(&mut reader(&msg)).is_none());
    }
}
