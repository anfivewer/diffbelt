use clap::{Parser, Subcommand};

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

fn main() {
    let args = Args::parse();

    println!("Hello {:?}!", args)
}
