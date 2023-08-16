use clap::{Parser, Subcommand};
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

#[derive(Subcommand, Debug)]
enum CollectionsSubcommand {
    /// Lists collections
    List,
    /// Alias to list
    Ls,
}

async fn run() {
    let args = Args::parse();

    println!("Hello {:?}!", args);
}

fn main() {
    let runtime = create_main_tokio_runtime().unwrap();

    runtime.block_on(run());
}
