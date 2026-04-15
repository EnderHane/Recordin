mod cli;
mod webui;

fn main() -> color_eyre::Result<()> {
    color_eyre::install()?;

    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        // Run CLI logic
        let cli = clap::Parser::parse();
        cli::run(cli)?;
    } else {
        // Run Web UI logic
        webui::serve_webui()?;
    }

    Ok(())
}
