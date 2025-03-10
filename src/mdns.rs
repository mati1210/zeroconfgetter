// SPDX-License-Identifier: MPL-2.0

use std::sync::Arc;

use crate::Hosts;
use mdns_sd::{IfKind, ServiceDaemon, ServiceEvent, ServiceInfo};
use tokio::spawn;

pub async fn listener(hosts: Hosts) {
    let prefer_ipv6 = std::env::var_os("PREFER_IPV6").is_some();

    let service = crate::die!({ ServiceDaemon::new()} "failed to create mdns daemon! {err}");
    crate::die!({service.enable_interface(IfKind::All)} "failed to enable all interfaces! {err}");
    let browser = crate::die!({service.browse("_ssh._tcp.local.")} "failed to browse mdns! {err}");

    while let Ok(event) = browser.recv_async().await {
        if let ServiceEvent::ServiceResolved(info) = event {
            let _hosts = Arc::clone(&hosts);
            spawn(async move { handler(info, _hosts, prefer_ipv6).await });
        }
    }
}

async fn handler(service: ServiceInfo, hosts: Hosts, prefer_ipv6: bool) {
    let mut address: Option<String> = None;
    for addr in service.get_addresses() {
        if addr.is_loopback()
            || if prefer_ipv6 {
                addr.is_ipv4()
            } else {
                addr.is_ipv6()
            }
        {
            continue;
        }

        address = Some(addr.to_string());
        break;
    }
    let Some(address) = address else { return };
    let Some(host_prop) = service.get_property("host") else {
        return;
    };

    let hosts_ro = hosts.read().await;
    if hosts_ro.get(host_prop.val_str()) == Some(&address) {
        return;
    }
    drop(hosts_ro);

    let mut rw_lock = hosts.write().await;
    rw_lock.insert(host_prop.val_str().to_string(), address.clone());
}
