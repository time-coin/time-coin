use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "proposal-cli")]
#[command(about = "TIME Coin Proposal Management CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new proposal
    Create {
        #[arg(short, long)]
        title: String,
        
        #[arg(short, long)]
        amount: u64,
    },
    
    /// Submit a proposal
    Submit {
        #[arg(short, long)]
        id: String,
    },
    
    /// List all proposals
    List,
    
    /// Get proposal details
    Get {
        #[arg(short, long)]
        id: String,
    },
}

fn main() {
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Create { title, amount } => {
            println!("Creating proposal: {} for {} TIME", title, amount);
        }
        Commands::Submit { id } => {
            println!("Submitting proposal: {}", id);
        }
        Commands::List => {
            println!("Listing all proposals...");
        }
        Commands::Get { id } => {
            println!("Getting proposal details: {}", id);
        }
    }
}
