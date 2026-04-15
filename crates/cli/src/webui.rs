use std::{
    io::{
        Read,
        Write,
    },
    net::{
        TcpListener,
        TcpStream,
    },
    process::Command,
};

use crate::cli::{
    Cli,
    Graphics,
    Sound,
};

pub fn serve_webui() -> color_eyre::Result<()> {
    let start_port = 43009;
    let mut port = start_port;
    let listener = loop {
        if let Ok(l) = TcpListener::bind(format!("127.0.0.1:{}", port)) {
            break l;
        }
        port += 1;
        if port > start_port + 100 {
            // Fallback to OS choice if range exhausted
            break TcpListener::bind("127.0.0.1:0")?;
        }
    };
    let host = listener.local_addr()?.ip();
    let port = listener.local_addr()?.port();
    let url = format!("http://{}:{}", host, port);

    println!("🌐 Web UI available at: {}", url);
    println!("Press Ctrl+C or use the Quit button in the browser to exit.");

    open_browser(&url);

    for stream in listener.incoming() {
        let stream = stream?;
        if handle_connection(stream)?.is_some() {
            break;
        }
    }

    Ok(())
}

fn open_browser(url: &str) {
    let _ = Command::new("cmd").args(["/C", "start", url]).spawn();
}

fn handle_connection(mut stream: TcpStream) -> color_eyre::Result<Option<()>> {
    let mut buffer = [0; 4096];
    let n = stream.read(&mut buffer)?;
    let request = String::from_utf8_lossy(&buffer[..n]);

    if request.starts_with("GET / ") {
        let html_content = include_str!("webui.html");
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
            html_content.len(),
            html_content
        );
        stream.write_all(response.as_bytes())?;
        return Ok(None);
    } else if request.starts_with("POST /start") {
        let body = request.split("\r\n\r\n").nth(1).unwrap_or("");
        let cli = parse_form_to_cli(body);

        // Run the CLI logic
        crate::cli::run(cli)?;

        let response_html = r#"
        <html><body style="font-family:sans-serif; text-align:center; padding:50px;">
        <h1>✅ Started!</h1><p>The executable has been launched.</p>
        <p><a href="/">← Back</a></p></body></html>
        "#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
            response_html.len(),
            response_html
        );
        stream.write_all(response.as_bytes())?;
        // We don't necessarily exit here so the user can "Quit" or "Start" again
        return Ok(Some(()));
    } else if request.starts_with("POST /quit") {
        let response_html = r#"
        <html><body style="font-family:sans-serif; text-align:center; padding:50px;">
        <h1>👋 Goodbye!</h1><p>The controller is shutting down.</p></body></html>
        "#;
        let response = format!(
            "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
            response_html.len(),
            response_html
        );
        stream.write_all(response.as_bytes())?;
        return Ok(Some(()));
    }

    Ok(None)
}

fn parse_form_to_cli(body: &str) -> Cli {
    let mut fps = 60.0;
    let mut graphics = Graphics::default();
    let mut sound = Sound::default();
    let mut video_encoder = None;
    let mut video_option = None;
    let mut video_output = None;
    let mut audio_output = None;
    let mut target_regex = None;
    let mut aggressive_infect = false;
    let mut force_tick_threshold = None;
    let mut executable = String::new();
    let mut exec_args = Vec::new();

    for pair in body.split('&') {
        let mut parts = pair.splitn(2, '=');
        let key = parts.next().unwrap_or("");
        let value = parts.next().unwrap_or("");
        let decoded_value = percent_decode(value);

        match key {
            "fps" => fps = decoded_value.parse().unwrap_or(60.0),
            "graphics_hook" => match decoded_value.as_str() {
                "vulkan" => graphics.vulkan = true,
                "d3d11" => graphics.d3d11 = true,
                _ => {}
            },
            "wasapi" if decoded_value == "on" => {
                sound.wasapi = true;
            }
            "aggressive_infect" => aggressive_infect = decoded_value == "on",
            "target_regex" => target_regex = Some(decoded_value),
            "force_tick_threshold" => force_tick_threshold = decoded_value.parse().ok(),
            "video_encoder" => video_encoder = Some(decoded_value),
            "video_option" => video_option = Some(decoded_value),
            "video_output" => video_output = Some(decoded_value),
            "audio_output" => audio_output = Some(decoded_value),
            "executable" => executable = decoded_value,
            "exec_args" => exec_args = decoded_value.split_whitespace().map(String::from).collect(),
            _ => {}
        }
    }

    Cli {
        fps,
        graphics,
        sound,
        video_encoder,
        video_option,
        video_output,
        audio_output,
        target_regex,
        aggressive_infect,
        force_tick_threshold,
        executable,
        exec_args,
    }
}

fn percent_decode(input: &str) -> String {
    let mut output = String::new();
    let mut chars = input.chars();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                output.push(byte as char);
            }
        } else if c == '+' {
            output.push(' ');
        } else {
            output.push(c);
        }
    }
    output
}
