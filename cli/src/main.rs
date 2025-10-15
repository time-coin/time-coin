use clap::Parser;

#[derive(Parser)]
#[command(name = "time-node")]
#[command(about = "TIME Coin Node", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Parser)]
enum Commands {
    /// Start the node
    Start,
    /// Get node status
    Status,
    /// Initialize configuration
    Init,
}

fn main() {
    let cli = Cli::parse();

    match cli.command {
        Commands::Start => {
            println!("Starting TIME node...");
        }
        Commands::Status => {
            println!("Checking node status...");
        }
        Commands::Init => {
            println!("Initializing node configuration...");
        }
    }
}
