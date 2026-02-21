mod commands;
mod output;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "rotary",
    version,
    about = "Audit secret health across your vaults"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scan secret sources and print a health report.
    ///
    /// With no flags, reads sources from rotary.toml in the current or
    /// parent directories. Use --source to scan a single source directly.
    Scan {
        /// Scan a specific source type (e.g. "dotenv") instead of using rotary.toml.
        #[arg(short, long)]
        source: Option<String>,

        /// Path to the .env file (required when --source=dotenv).
        #[arg(short, long)]
        path: Option<String>,

        /// Environment label (e.g. "production", "staging").
        #[arg(short, long)]
        env: Option<String>,

        /// Maximum secret age in days before flagging as critical.
        #[arg(long)]
        max_age: Option<u64>,

        /// Output as JSON instead of the formatted report.
        #[arg(long)]
        json: bool,
    },

    /// Show detailed info about a specific secret key.
    ///
    /// Looks up the key across configured sources and displays metadata,
    /// health status, and a matching rotation playbook if one exists.
    Details {
        /// The secret key name to look up (e.g. "STRIPE_SECRET_KEY").
        key: String,

        /// Source type to search in (uses rotary.toml if omitted).
        #[arg(short, long)]
        source: Option<String>,

        /// Path to the .env file (required when --source=dotenv).
        #[arg(short, long)]
        path: Option<String>,

        /// Environment label.
        #[arg(short, long)]
        env: Option<String>,
    },

    /// Create a starter rotary.toml in the current directory.
    Init,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::from_default_env()
                .add_directive("rotary=info".parse().unwrap()),
        )
        .without_time()
        .init();

    let cli = Cli::parse();

    let result = match cli.command {
        Commands::Scan {
            source,
            path,
            env,
            max_age,
            json,
        } => {
            commands::scan::run(
                source.as_deref(),
                path.as_deref(),
                env.as_deref(),
                max_age,
                json,
            )
            .await
        }
        Commands::Details {
            key,
            source,
            path,
            env,
        } => commands::details::run(&key, source.as_deref(), path.as_deref(), env.as_deref()).await,
        Commands::Init => {
            let cwd = std::env::current_dir().unwrap_or_else(|_| ".".into());
            commands::init::run(&cwd)
        }
    };

    if let Err(e) = result {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
