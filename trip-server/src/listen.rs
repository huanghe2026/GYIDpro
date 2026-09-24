//! 监听目标解析与启动。
//!
//! `TRIP_LISTEN` 支持两种形态：
//!
//! - `<ip>:<port>`（如 `127.0.0.1:8080`）—— 传统 TCP；
//! - `unix:<path>`（如 `unix:/run/trip-server.sock`）—— Unix domain socket。
//!
//! 生产部署（`gyid.geoyuan.com`）使用后者：进程只在文件系统里留下一个 socket
//! 文件，**不占用任何 TCP 端口**，因此不会与同机其他服务抢端口；nginx 侧用
//! `proxy_pass http://unix:/run/trip-server.sock:/;` 反代。
//!
//! # 为什么不用 `axum::serve`
//!
//! axum 0.7 的 `axum::serve` 签名固定为 `serve(TcpListener, M)`，没有可插拔的
//! `Listener` trait（该 trait 到 axum 0.8 才引入）。因此 Unix socket 这一路
//! 手动驱动 hyper 1.x：每条连接一个 task，并用 `with_upgrades()` 保证
//! `WS /v1/challenge` 的 101 升级可用。TCP 一路仍走 `axum::serve`。

use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use axum::Router;

/// `TRIP_LISTEN` 的解析结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ListenTarget {
    /// TCP 监听。
    Tcp(SocketAddr),
    /// Unix domain socket 监听。
    Unix(PathBuf),
}

impl ListenTarget {
    /// 解析 `TRIP_LISTEN` 的值。
    ///
    /// `unix:` 前缀表示 Unix domain socket（取前缀之后的路径，允许含空白）；
    /// 其余一律按 `<ip>:<port>` 解析——**不接受主机名**，避免启动期 DNS 依赖。
    pub fn parse(raw: &str) -> anyhow::Result<Self> {
        let s = raw.trim();
        if let Some(rest) = s.strip_prefix("unix:") {
            let path = rest.trim();
            if path.is_empty() {
                anyhow::bail!(
                    "TRIP_LISTEN `unix:` requires a socket path, e.g. unix:/run/trip-server.sock"
                );
            }
            return Ok(ListenTarget::Unix(PathBuf::from(path)));
        }
        let addr = s.parse::<SocketAddr>().map_err(|e| {
            anyhow::anyhow!("TRIP_LISTEN must be `<ip>:<port>` or `unix:<path>`, got {s:?}: {e}")
        })?;
        Ok(ListenTarget::Tcp(addr))
    }

    /// 人类可读描述（日志与错误信息用）。
    pub fn describe(&self) -> String {
        match self {
            ListenTarget::Tcp(addr) => addr.to_string(),
            ListenTarget::Unix(path) => format!("unix:{}", path.display()),
        }
    }

    /// Unix socket 路径；TCP 目标返回 `None`。
    pub fn unix_path(&self) -> Option<&Path> {
        match self {
            ListenTarget::Tcp(_) => None,
            ListenTarget::Unix(path) => Some(path.as_path()),
        }
    }
}

/// 在指定目标上启动服务，直到进程被终止。
pub async fn serve(target: ListenTarget, app: Router) -> anyhow::Result<()> {
    match target {
        ListenTarget::Tcp(addr) => {
            let listener = tokio::net::TcpListener::bind(addr).await?;
            tracing::info!(addr = %addr, "trip-server listening (tcp)");
            axum::serve(listener, app).await?;
            Ok(())
        }
        ListenTarget::Unix(path) => serve_unix(path, app).await,
    }
}

/// 绑定 Unix socket 并循环接受连接。
#[cfg(unix)]
async fn serve_unix(path: PathBuf, app: Router) -> anyhow::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    use std::time::Duration;

    if let Some(dir) = path.parent() {
        if !dir.as_os_str().is_empty() {
            std::fs::create_dir_all(dir)?;
        }
    }

    // 上次异常退出会留下 socket 文件，此时 bind 直接报 EADDRINUSE。
    // 先探测是否真有进程在监听：连得上说明有人服务，明确失败；连不上说明是残留，删掉。
    if path.exists() {
        match tokio::net::UnixStream::connect(&path).await {
            Ok(_) => anyhow::bail!(
                "{} is already being served by another process",
                path.display()
            ),
            Err(_) => std::fs::remove_file(&path)?,
        }
    }

    let listener = tokio::net::UnixListener::bind(&path)?;
    // trip-server 与 nginx 同属 www-data：组内可读写即可；
    // 用 0660 而非 0600，方便运维脚本以组身份直连排查。
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o660))?;
    tracing::info!(socket = %path.display(), "trip-server listening (unix socket)");

    loop {
        let (stream, _) = match listener.accept().await {
            Ok(pair) => pair,
            Err(err) => {
                // 单次 accept 失败（瞬时 fd 耗尽等）不该终止整个服务。
                tracing::warn!(%err, "unix socket accept failed");
                tokio::time::sleep(Duration::from_millis(50)).await;
                continue;
            }
        };
        let router = app.clone();
        tokio::spawn(async move { serve_connection(stream, router).await });
    }
}

/// 非 Unix 平台不支持 Unix socket 监听（服务端实际只跑 Linux）。
#[cfg(not(unix))]
async fn serve_unix(path: PathBuf, _app: Router) -> anyhow::Result<()> {
    anyhow::bail!(
        "`unix:` listen targets require a Unix platform (got {})",
        path.display()
    )
}

/// 在一条 Unix 连接上跑完一个 HTTP/1.1 会话（含 WebSocket 升级）。
#[cfg(unix)]
async fn serve_connection(stream: tokio::net::UnixStream, router: Router) {
    use hyper::body::Incoming;
    use hyper::service::service_fn;
    use hyper_util::rt::TokioIo;
    use tower::Service;

    let io = TokioIo::new(stream);
    // 每条连接自带一个 service；keep-alive 复用同一闭包，故只需 `Fn`。
    let service = service_fn(move |req: hyper::Request<Incoming>| {
        let mut router = router.clone();
        async move { router.call(req.map(axum::body::Body::new)).await }
    });

    // `with_upgrades()` 是 WebSocket 的前提：101 升级需要 hyper 交出底层连接。
    if let Err(err) = hyper::server::conn::http1::Builder::new()
        .serve_connection(io, service)
        .with_upgrades()
        .await
    {
        tracing::debug!(%err, "unix socket connection closed with error");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_tcp_target_and_trims() {
        assert_eq!(
            ListenTarget::parse("127.0.0.1:8080").expect("tcp"),
            ListenTarget::Tcp("127.0.0.1:8080".parse().expect("addr"))
        );
        assert_eq!(
            ListenTarget::parse("  0.0.0.0:9000  ").expect("trimmed"),
            ListenTarget::Tcp("0.0.0.0:9000".parse().expect("addr"))
        );
    }

    #[test]
    fn parses_unix_target() {
        let t = ListenTarget::parse("unix:/run/trip-server.sock").expect("unix");
        assert_eq!(t.describe(), "unix:/run/trip-server.sock");
        assert_eq!(
            t.unix_path(),
            Some(Path::new("/run/trip-server.sock")),
            "socket path must be preserved verbatim"
        );
        let t = ListenTarget::parse("unix:/tmp/a b.sock").expect("unix with space");
        assert_eq!(t.unix_path(), Some(Path::new("/tmp/a b.sock")));
    }

    #[test]
    fn rejects_malformed_targets() {
        for bad in ["", "   ", "8080", "localhost:8080", "unix:", "unix:   "] {
            assert!(
                ListenTarget::parse(bad).is_err(),
                "{bad:?} should be rejected"
            );
        }
    }

    #[test]
    fn tcp_target_has_no_unix_path() {
        let t = ListenTarget::parse("127.0.0.1:1").expect("tcp");
        assert!(t.unix_path().is_none());
        assert_eq!(t.describe(), "127.0.0.1:1");
    }
}
