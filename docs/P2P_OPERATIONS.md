# GeoYuan P2P 运维指南

> 版本: 0.1.0 · 最后更新: 2026-05-09

---

## 目录

1. [节点类型](#1-节点类型)
2. [部署步骤](#2-部署步骤)
3. [日常运维](#3-日常运维)
4. [故障排查](#4-故障排查)
5. [网络拓扑](#5-网络拓扑)
6. [性能监控](#6-性能监控)

---

## 1. 节点类型

| 类型 | 描述 | 资源需求 | 运行方式 |
|------|------|----------|----------|
| **Bootstrap 节点** | 网络入口，帮助新节点发现其他节点 | ~80-150MB | Linux 服务器，systemd 服务 |
| **CLI 节点** | 命令行 P2P 节点 | ~80-150MB | `geoyuan p2p start` |
| **GUI 节点** | 带图形界面的完整节点 | ~200-350MB | `geoyuan-gui.exe` |

### 端口规划

```
P2P 节点:   4001-4999  (libp2p TCP, 默认 4001)
WebSocket:  由 NGINX 转发到 4001
```

### 内存约束

所有节点类型均满足 ≤500MB 内存要求：
- CLI 节点: **~80-150MB** ✅
- GUI 节点: **~200-350MB** ✅

---

## 2. 部署步骤

### 2.1 首次部署（服务器端）

```bash
# 方式 A: 在服务器上直接编译（推荐）
cd geoyuan-web
bash deploy-p2p.sh --server-build

# 方式 B: 从本地上传预编译的 Linux 二进制
# 先在本机 WSL2 或 Docker 中编译:
#   cd /path/to/geoyuan && cargo build --release -p geoyuan-cli
# 然后上传:
bash deploy-p2p.sh --local-bin ./target/release/geoyuan
```

### 2.2 本地测试

```bash
# Windows
.\tools\p2p-test.ps1 -Build

# Linux
bash tools/p2p-test.sh --build
```

脚本会自动启动 3 个 P2P 节点，验证 mDNS 发现、GeoCast 广播和 PoL 位置证明。

### 2.3 手动启动节点

```bash
# 默认端口（自动选择 4001-4999）
geoyuan p2p start

# 指定端口
geoyuan p2p start --port 4001

# 查询状态
geoyuan p2p status

# 列出对等节点
geoyuan p2p peers

# 查询地理邻近节点（需 H3 cell）
geoyuan p2p peers-nearby 8a1fb46622dffff
```

---

## 3. 日常运维

### 3.1 服务管理

```bash
# 状态
systemctl status geoyuan-p2p

# 启动/停止/重启
systemctl start geoyuan-p2p
systemctl stop geoyuan-p2p
systemctl restart geoyuan-p2p

# 开机自启
systemctl enable geoyuan-p2p

# 查看日志
journalctl -u geoyuan-p2p -f
journalctl -u geoyuan-p2p --since "5 minutes ago"
```

### 3.2 升级节点

```bash
# 重新编译并替换二进制
bash deploy-p2p.sh --server-build

# 或手动替换二进制后重启
systemctl restart geoyuan-p2p
```

### 3.3 备份

P2P 节点数据存储在 `~/.local/share/geoyuan/` 目录下：
```bash
# 备份
tar czf geoyuan-backup-$(date +%Y%m%d).tar.gz ~/.local/share/geoyuan/

# 恢复
tar xzf geoyuan-backup-*.tar.gz -C ~/
```

---

## 4. 故障排查

### 4.1 节点无法启动

```bash
# 检查 systemd 日志
journalctl -u geoyuan-p2p -n 50 --no-pager

# 检查端口是否被占用
ss -tlnp | grep 4001

# 手动运行测试（非 systemd）
/usr/local/bin/geoyuan p2p start --port 4001
```

### 4.2 节点间无法互相发现

| 原因 | 检查方法 | 解决方法 |
|------|----------|----------|
| 防火墙阻止端口 | `iptables -L -n 检查 4001 端口` | 开放端口 |
| mDNS 不生效 | 检查节点是否在同一局域网 | 跨公网需 Kademlia DHT |
| 网络不可达 | `ping 对方 IP` | 检查路由/防火墙 |
| Kademlia 没有 bootstrap | `geoyuan p2p status` 检查 peer_count | 确保至少有一个 bootstrap 节点在线 |

### 4.3 GeoCast 广播失败

```bash
# 检查节点是否设置了 H3 cell
geoyuan p2p status

# 检查 H3 cell 格式（16 进制）
# 有效示例: 8a1fb46622dffff

# 检查邻居 ring 范围（默认 3 环，~55m 半径）
# 两个节点的 GPS 位置必须在 ~55m 范围内
```

### 4.4 PoL 验证失败

```bash
# PoL 验证失败的常见原因:
# 1. GPS 坐标精度不足
# 2. Ed25519 签名公钥不匹配
# 3. 时间戳偏差过大（>5 分钟）
# 4. payload hash 不匹配
```

### 4.5 NGINX 配置问题

```bash
# 检查 NGINX 配置
nginx -t

# 检查 P2P WebSocket 代理日志
tail -f /var/log/nginx/p2p-ws-error.log

# 检查 NGINX 是否加载了 geoyuan-p2p 配置
ls -la /etc/nginx/conf.d/geoyuan-p2p.conf
```

### 4.6 常见错误码

| 日志内容 | 含义 | 处理方式 |
|----------|------|----------|
| `Connection timed out` | 对端不可达 | 检查网络/防火墙 |
| `IO error on outbound stream` | 流连接错误 | 可能是协议不匹配 |
| `Semantic(None, ...)` | CBOR 解码失败 | 协议版本不兼容 |
| `The response channel was dropped` | 响应未发送 | 通常无害，节点主动断开 |

---

## 5. 网络拓扑

```
                    GeoYuan P2P 网络
                        │
              ┌─────────┴─────────┐
              │                   │
    ┌──── Bootstrap 节点 ────┐   │
    │  <SERVER_IP>:4001      │   │
    │  systemd 守护          │   │
    └────────┬───────────────┘   │
             │ mDNS/Kademlia     │
    ┌────────┴────────┐   ┌─────┴──────┐
    │ Windows CLI 节点 │   │ Windows GUI│
    │  192.168.x.x    │   │  节点       │
    └─────────────────┘   └────────────┘
```

### 5.1 协议栈

```
GPv1 — Geospatial Proof Protocol
├── PoL 验证层 (GPS + Ed25519 签名)
├── GeoCast 路由层 (H3 Res 12, ~9m 边长)
└── libp2p 传输层 (TCP/Noise/Yamux/Kademlia/mDNS)
```

### 5.2 消息类型

| 类型 | 描述 | 需 PoL |
|------|------|--------|
| `GeoCast` | 地理广播消息 | ✅ 必须 |
| `CoinTransferred` | 转账广播 | ✅ 建议 |
| `WalletRequest` | 钱包同步请求 | ❌ 可选 |
| `Ping/Pong` | 心跳 | ❌ |

---

## 6. 性能监控

### 6.1 关键指标

| 指标 | 正常范围 | 告警阈值 |
|------|----------|----------|
| 内存占用 | 80-150 MB | >300 MB |
| CPU 使用率 | <5% | >30% |
| 连接节点数 | 1-20 | 0（超过 30 秒） |
| 入站消息速率 | 视网络情况 | - |

### 6.2 健康检查

```bash
# 快速检查脚本
cat << 'SCRIPT' > /usr/local/bin/check-geoyuan.sh
#!/bin/bash
if systemctl is-active --quiet geoyuan-p2p; then
    PEERS=$(journalctl -u geoyuan-p2p --no-pager -n 1 | grep -oP '连接数=\K\d+' || echo "0")
    echo "OK peer_count=$PEERS"
    exit 0
else
    echo "CRITICAL geoyuan-p2p not running"
    exit 2
fi
SCRIPT
chmod +x /usr/local/bin/check-geoyuan.sh
```

### 6.3 集成到 Prometheus + Grafana（可选）

```bash
# geoyuan-p2p 暴露 metrics 端点（后续版本）
# 在 /etc/prometheus/prometheus.yml 中添加:
#   - job_name: 'geoyuan-p2p'
#     static_configs:
#       - targets: ['localhost:4001']
```

---

## 附录

### A. 部署文件清单

```
geoyuan/
├── tools/
│   ├── p2p-test.sh          # Linux 本地测试脚本
│   └── p2p-test.ps1         # Windows 本地测试脚本
├── deploy/
│   ├── geoyuan-p2p.service   # systemd 服务配置
│   └── nginx-geoyuan-p2p.conf # NGINX WSS 代理配置
├── geoyuan-web/
│   └── deploy-p2p.sh        # 一键部署脚本
└── docs/
    └── P2P_OPERATIONS.md    # 本文件
```

### B. 相关链接

- 服务器: `<SERVER_IP>`（见 deploy 脚本 --server 参数或 DEPLOY_SERVER 环境变量）
- 网站: `https://www.geoyuan.com`
- P2P 节点日志: `journalctl -u geoyuan-p2p -f`
