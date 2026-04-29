mod memory;
mod paths;
mod render;
mod tree;

use std::path::PathBuf;
use std::process::Command;

use tiny_http::{Header, Response, Server};

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}

fn bind_ephemeral() -> Result<(Server, u16), String> {
    let server = Server::http("127.0.0.1:0").map_err(|e| e.to_string())?;
    let port = server
        .server_addr()
        .to_ip()
        .map(|s| s.port())
        .ok_or_else(|| "no IP socket address available".to_string())?;
    Ok((server, port))
}

fn main() {
    let home = match home_dir() {
        Some(h) => h,
        None => {
            eprintln!("Could not determine $HOME.");
            std::process::exit(1);
        }
    };

    let (server, port) = match bind_ephemeral() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Failed to bind: {e}");
            std::process::exit(1);
        }
    };

    let url = format!("http://127.0.0.1:{port}");
    println!("Auto-memory viewer at {url}");
    if let Err(e) = Command::new("open").arg(&url).status() {
        eprintln!("(could not auto-open browser: {e}; visit the URL above)");
    }

    for request in server.incoming_requests() {
        if request.url() != "/" {
            let _ = request.respond(Response::from_string("not found").with_status_code(404));
            continue;
        }
        let projects = memory::scan_all(&home);
        let tree = tree::build_tree(&projects);
        let html = render::render_page(&tree, &projects);
        let header = Header::from_bytes(
            &b"Content-Type"[..],
            &b"text/html; charset=utf-8"[..],
        )
        .unwrap();
        let resp = Response::from_string(html).with_header(header);
        let _ = request.respond(resp);
    }
}
