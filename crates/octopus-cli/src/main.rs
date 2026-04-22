#[tokio::main]
async fn main() {
    if let Err(error) = octopus_cli::run_once::main_from_env().await {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
