// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use crate::Hosts;
use tokio::spawn;
use zeroconf_tokio::{prelude::*, MdnsBrowser, MdnsBrowserAsync, ServiceDiscovery, ServiceType};

pub async fn listener(hosts: Hosts) {
    let prefer_ipv6 = std::env::var_os("PREFER_IPV6").is_some();

    let service = ServiceType::new("ssh", "tcp").unwrap();
    let mut browser = crate::die!({MdnsBrowserAsync::new(MdnsBrowser::new(service)) } "failed creating mdns browser! {err}");
    crate::die!( {browser.start().await } "failed starting mdns browser! {err}");
    while let Some(Ok(discovery)) = browser.next().await {
        let _hosts = Arc::clone(&hosts);
        spawn(async move { handler(discovery, _hosts, prefer_ipv6).await });
    }
}

async fn handler(service: ServiceDiscovery, hosts: Hosts, prefer_ipv6: bool) {
    let address = service.address();
    if address == "127.0.0.1" || address.contains(if prefer_ipv6 { '.' } else { ':' }) {
        return;
    }

    let Some(txt) = service.txt() else {
        return;
    };

    let Some(host) = txt.get("host") else {
        return;
    };

    let ro_lock = hosts.read().await;
    if ro_lock.get(&host) == Some(address) {
        return;
    }
    drop(ro_lock);

    let mut rw_lock = hosts.write().await;
    rw_lock.insert(host, address.clone());
}
