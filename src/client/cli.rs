use std::path::PathBuf;

use clap::{value_parser, Arg, Command};
use dusa_common::VERSION;

pub fn build_cli() -> Command {
    Command::new("Dusa")
        .about("A recs based encryption client / server")
        .version(VERSION)
        .trailing_var_arg(true)
        .arg(
            Arg::new("path")
                .short('p')
                .long("path")
                .value_parser(value_parser!(PathBuf))
                .help("filepath to encrypt")
                .num_args(1),
        )
        .arg(
            Arg::new("owner")
                .short('o')
                .long("owner")
                .value_parser(value_parser!(String))
                .help("The owner of encrypted file or the owner of the file to be encrypted")
                .num_args(1),
        )
        .arg(
            Arg::new("name")
                .short('n')
                .long("name")
                .value_parser(value_parser!(String))
                .help("The name of the file to be encrypted or already encrypted")
                .num_args(1),
        )
        .arg(
            Arg::new("encrypt_file")
                .long("ef")
                .action(clap::ArgAction::SetTrue)
                .help("Encrypting file"),
        )
        .arg(
            Arg::new("encrypt_text")
                .long("et")
                .action(clap::ArgAction::SetTrue)
                .help("Encrypt text data"),
        )
        .arg(
            Arg::new("decrypt_file")
                .long("df")
                .action(clap::ArgAction::SetTrue)
                .help("Decrypt file"),
        )
        .arg(
            Arg::new("decrypt_text")
                .long("dt")
                .action(clap::ArgAction::SetTrue)
                .help("Decrypt text data"),
        )
        .arg(
            Arg::new("remove_file")
                .long("rf")
                .action(clap::ArgAction::SetTrue)
                .help("Remove encrypted file"),
        )
        .arg(
            Arg::new("data")
                .short('d')
                .long("data")
                .value_parser(value_parser!(String))
                .help("plain text data to encrypt or decrypt")
                .num_args(1),
        )
}
