use crate::commands::transform::run::RunSubcommand;
use clap::error::ErrorKind;
use clap::{ArgMatches, Command, Error, FromArgMatches, Subcommand};

use crate::global::get_global_config;

impl FromArgMatches for RunSubcommand {
    fn from_arg_matches(matches: &ArgMatches) -> Result<Self, Error> {
        let Some((name, _)) = matches.subcommand() else {
            return Err(Error::new(ErrorKind::DisplayHelp));
        };

        Ok(RunSubcommand {
            name: name.to_string(),
        })
    }

    fn update_from_arg_matches(&mut self, _matches: &ArgMatches) -> Result<(), Error> {
        todo!()
    }
}

impl Subcommand for RunSubcommand {
    fn augment_subcommands(cmd: Command) -> Command {
        let config = get_global_config();

        let Some(config) = config else {
            return cmd.subcommand(
                Command::new("no_transforms_available")
                    .about("Add transforms to the config or/and add \"name\" property to them"),
            );
        };

        let mut cmd = cmd;

        for name in config.transform_names() {
            if name == "help" {
                // Clap does not allows to override help
                continue;
            }

            cmd = cmd.subcommand(Command::new(name.to_string()));
        }

        cmd
    }

    fn augment_subcommands_for_update(_cmd: Command) -> Command {
        todo!()
    }

    fn has_subcommand(_name: &str) -> bool {
        todo!()
    }
}
