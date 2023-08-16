use clap::{Parser, Subcommand};
use diffbelt_http_client::client::{DiffbeltClient, DiffbeltClientNewOptions};
use diffbelt_util::tokio_runtime::create_main_tokio_runtime;

#[derive(Parser, Debug)]
#[command()]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Collections {
        #[command(subcommand)]
        command: CollectionsSubcommand,
    },
}

impl Commands {
    async fn run(&self, client: &DiffbeltClient) {
        match self {
            Commands::Collections { command } => {
                command.run(client).await
            }
        }
    }
}

#[derive(Subcommand, Debug)]
enum CollectionsSubcommand {
    /// Lists collections
    List,
    /// Alias to list
    Ls,
}

impl CollectionsSubcommand {
    async fn run(&self, client: &DiffbeltClient) {
        let response = client.list_collections().await.unwrap();

        for item in response.items {
            println!("{} {}", item.name, if item.is_manual { "manual" } else { "non-manual" });
        }
    }
}

async fn run() {
    let args = Args::parse();

    let client = DiffbeltClient::new(DiffbeltClientNewOptions {
        host: "127.0.0.1".to_string(),
        port: 3030,
    });

    args.command.run(&client).await;
}

fn main() {
    let runtime = create_main_tokio_runtime().unwrap();

    runtime.block_on(run());
}
