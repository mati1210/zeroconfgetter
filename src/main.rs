// SPDX-License-Identifier: MPL-2.0

use std::{collections::HashMap, sync::Arc};

use tokio::{spawn, sync::RwLock};
pub type Hosts = Arc<RwLock<HashMap<String, String>>>;

pub mod mdns;
pub mod socket;

#[macro_export]
macro_rules! die {
    ({ $($t:tt)* } $($msg:tt)* ) => {
        match{ $($t)* } {
        Ok(o) => o,
        Err(e) => {
            eprintln!($($msg)*, err = e);
            std::process::exit(1);
        }}

    };
}

#[tokio::main]
async fn main() {
    let hosts: Hosts = Default::default();

    let _hosts = Arc::clone(&hosts);
    spawn(async move { socket::listener(_hosts).await });

    mdns::listener(hosts).await
}
