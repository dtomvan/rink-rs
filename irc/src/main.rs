// This Source Code Form is subject to the terms of the Mozilla Public
// License, v. 2.0. If a copy of the MPL was not distributed with this
// file, You can obtain one at https://mozilla.org/MPL/2.0/.

use futures::StreamExt;
use irc::client::prelude::*;
use rink_sandbox::Alloc;

mod config;
mod service;

#[global_allocator]
pub(crate) static GLOBAL: Alloc = Alloc::new(usize::MAX);

#[tokio::main]
async fn main() {
    let config_file = std::fs::read_to_string("config.toml").unwrap();
    let config = toml::from_str::<config::Config>(&config_file).unwrap();

    println!("Loading rink...");
    let mut ctx = config::load(&config);

    println!("Making sure we can actually run queries...");
    let test_query = rink_core::eval(&mut ctx, "m");
    match test_query {
        Ok(_res) => (),
        bad => {
            println!("failed to run a basic query: {:#?}", bad);
            std::process::exit(1);
        }
    }

    // todo: multiple servers
    let server = &config.servers[0];
    let mut client = Client::from_config(server.clone()).await.unwrap();
    println!("Connecting...");
    client.identify().unwrap();
    println!("Connected. nickname: {}", client.current_nickname());

    let mut stream = client.stream().unwrap();
    while let Some(message) = stream.next().await.transpose().unwrap() {
        let source = message.source_nickname();
        if let Command::PRIVMSG(ref channel, ref text) = message.command {
            let nick = client.current_nickname();
            let prefix = format!("{}: ", nick);
            let command = if let Some(text) = text.strip_prefix(&prefix) {
                text
            } else if channel == client.current_nickname() {
                &text
            } else {
                continue;
            };
            println!("<== {command}");
            let result = rink_core::eval(&mut ctx, command);
            let result = match result {
                Ok(res) => format!("{}", res),
                Err(err) => format!("{}", err),
            };
            println!("==> {result}");
            let where_to = if channel == client.current_nickname() {
                if let Some(source) = source {
                    source
                } else {
                    continue;
                }
            } else {
                &channel
            };
            client.send_notice(where_to, result).unwrap();
        }
    }
}
