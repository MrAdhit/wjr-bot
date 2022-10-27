#![feature(async_closure)]

#[macro_use]
extern crate lazy_static;

mod api;
mod bot;

use dotenv::dotenv;

#[tokio::main]
async fn main() {
    dotenv().ok();

    bot::launch();

    loop {
        // TO THE INFINITY, AND BEYOND...
    }
}