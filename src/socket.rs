// SPDX-License-Identifier: MPL-2.0

static MAP: phf::Map<&str, &str> = phf_map! {
    "2015-000-0008266" =>"1 - ",
    "2015-000-0008285" =>"2 - ",
    "2015-000-0008268" =>"3 - ",
    "2015-000-0008284" =>"4 - ",
    "2015-000-0008291" =>"5 - ",
    "2015-000-0008272" =>"6 - ",
    "2015-000-0008286" =>"7 - ",
    "2015-000-0008294" =>"8 - ",
    "2015-000-0008290" =>"9 - ",
    "2015-000-0008288" => "10 - ",
    "2015-000-0008287" => "11 - ",
    "2015-000-0008265" => "12 - ",
    "2015-000-0008279" => "13 - ",
    "2015-000-0008289" => "14 - ",
    "2015-000-0008271" => "15 - ",
    "2015-000-0008267" => "16 - ",
    "2015-000-0008283" => "17 - ",
    "2015-000-0008277" => "18 - ",
    "2015-000-0008276" => "19 - ",
    "2015-000-0008292" => "20 - ",
    "2015-000-0008275" => "21 - ",
    "2015-000-0008293" => "22 - ",
    "2015-000-0008274" => "23 - ",
    "2015-000-0008269" => "24 - ",
    "2014-000-0112455" => "25 - ",
};

use crate::{die, Hosts};
use std::{ffi::OsString, fs, io, path::PathBuf, sync::Arc};

use phf::phf_map;
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
        if let Some(n) = MAP.get(host) {
            buf.write_all(n.as_bytes()).await?;
        }
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
