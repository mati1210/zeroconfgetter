// SPDX-License-Identifier: MPL-2.0

use crate::{die, Hosts};
use std::{ffi::OsString, fs, io, path::PathBuf, sync::Arc};

use tokio::{
    io::{AsyncWriteExt, BufWriter},
    net::{UnixListener, UnixStream},
    task::spawn,
};

pub async fn listener(hosts: Hosts) {
    let mut path = PathBuf::from(
        std::env::var_os("RUNTIME_DIRECTORY")
            .unwrap_or_else(|| OsString::from("/run/zeroconfgetter/")),
    );

    path.push("zeroconfgetter.sock");

    if die!({fs::exists(&path)} "failed checking if {} exists! {err}", path.display()) {
        die!({fs::remove_file(&path)} "failed removing {}! {err}", path.display())
    }

    let old_umask = unsafe { libc::umask(0) };
    let listener =
        die!( {UnixListener::bind(&path) } "failed binding to {}! {err}", path.display());
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
