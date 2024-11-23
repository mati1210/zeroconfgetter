// SPDX-License-Identifier: MPL-2.0

const SOCK_PATH: &str = "/run/zeroconfgetter/zeroconfgetter.sock";
use crate::Hosts;
use std::{fs, io, sync::Arc};

use tokio::{
    io::{AsyncWriteExt, BufWriter},
    net::{UnixListener, UnixStream},
    task::spawn,
};

pub async fn listener(hosts: Hosts) {
    if crate::die!({std::fs::exists(SOCK_PATH) } "failed checking if {SOCK_PATH} exists! {err}") {
        crate::die!( { fs::remove_file(SOCK_PATH) } "failed removing {SOCK_PATH}! {err}")
    }

    let old_umask = unsafe { libc::umask(0) };
    let listener =
        crate::die!( {UnixListener::bind(SOCK_PATH) } "failed binding to {SOCK_PATH}! {err}");
    unsafe { libc::umask(old_umask) };

    while let Ok((stream, _addr)) = listener.accept().await {
        let host = Arc::clone(&hosts);
        spawn(async move {
            if let Err(e) = handler(stream, host).await {
                eprintln!("socket fail! {e}")
            }
        });
    }
}

async fn handler(stream: UnixStream, hosts: Hosts) -> io::Result<()> {
    let mut buf = BufWriter::new(stream);

    let lock = hosts.read().await;
    let mut iter = lock.iter().peekable();
    while let Some((host, ip)) = iter.next() {
        buf.write_all(host.as_bytes()).await?;
        buf.write_all(b"\0").await?;
        buf.write_all(ip.as_bytes()).await?;
        if iter.peek().is_some() {
            buf.write_all(b"\0").await?;
        }
    }
    drop(lock);
    buf.flush().await?;

    Ok(())
}
