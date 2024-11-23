// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use crate::Hosts;
use tokio::spawn;
use zeroconf_tokio::{prelude::*, MdnsBrowser, MdnsBrowserAsync, ServiceDiscovery, ServiceType};

pub async fn listener(hosts: Hosts) {
    let service = ServiceType::new("ssh", "tcp").unwrap();
    let mut browser = crate::die!({MdnsBrowserAsync::new(MdnsBrowser::new(service)) } "failed creating mdns browser! {err}");
    crate::die!( {browser.start().await } "failed starting mdns browser! {err}");
    while let Some(Ok(discovery)) = browser.next().await {
        let _hosts = Arc::clone(&hosts);
        spawn(async move { handler(discovery, _hosts).await });
    }
}

async fn handler(service: ServiceDiscovery, hosts: Hosts) {
    if service.address() == "127.0.0.1" || service.address().contains(':') {
        return;
    }

    let Some(txt) = service.txt() else {
        return;
    };

    let Some(host) = txt.get("host") else {
        return;
    };

    let ro_lock = hosts.read().await;
    if ro_lock.get(&host) == Some(service.address()) {
        return;
    }
    drop(ro_lock);

    let mut rw_lock = hosts.write().await;
    rw_lock.insert(host, service.address().clone());
}
