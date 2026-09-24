# W7 标定数据准备

本目录只放**数据获取与自检脚本**。统计核心在 `trip-core::engine::calibration`
（与线上判定共用同一份引擎实现），命令行入口是 `gyid calibrate`。

## 目录约定

```
tools/calibration/
├── download_geolife.sh   # 下载 + 完整性测试 + 解压到 data/
├── prepare_dataset.sh    # 校验 <user>/Trajectory/*.plt 结构并输出摘要
├── .gitignore            # 忽略 data/ 与 *.zip（原始数据不入库）
└── data/                 # ← 数据落在这里（被 git 忽略）
```

`data/` 与所有 `*.zip` 已在 `.gitignore` 中，**不要把数据集提交进仓库**。

## 数据集

| 数据集 | 内容 | 用途 |
|---|---|---|
| **GeoLife 1.3** | 北京 182 用户、约 1.7 万条 GPS 轨迹 | 主标定集（中国城市人群） |
| MDC | 洛桑多模态移动数据（欧洲人群） | 跨人群对照（后续） |
| T-Drive | 北京出租车轨迹 | 职业偏差对照（仅作参考，不代表一般人群） |

- 官方来源：<https://www.microsoft.com/en-us/download/details.aspx?id=52367>
- 许可：Microsoft Research 数据集，**仅供研究/非商业用途**。使用前请阅读官方许可条款。
- 本仓库**不附带**任何原始数据。

## 快速开始

```bash
# 1) 下载并解压（约 1.9 GB；可用 GEOLIFE_URL 指定镜像）
tools/calibration/download_geolife.sh

# 2) 校验结构（打印用户数 / 轨迹数 / 磁盘占用，不读取坐标）
tools/calibration/prepare_dataset.sh --root "tools/calibration/data/GeoLife Trajectories 1.3"

# 3) 运行标定（--control 200 会为三族各生成 200 条合成攻击以计算 AUC）
cargo build --release -p trip-cli
./target/release/gyid calibrate geolife \
  --data-dir "tools/calibration/data/GeoLife Trajectories 1.3" \
  --control 200 \
  --output /tmp/geolife.json

# 4) 渲染白皮书草稿
./target/release/gyid calibrate whitepaper --input /tmp/geolife.json \
  --output docs/CALIBRATION-W7-WHITEPAPER.md
```

**无需下载也能跑通全链路**（用于 CI / 快速自检）：

```bash
./target/release/gyid calibrate synth --humans 60 --control 200 --output /tmp/synth.json
./target/release/gyid calibrate whitepaper --input /tmp/synth.json --output /tmp/synth-wp.md
```

`synth` 用 `engine::sim` 生成结构化 Levy"人类"轨迹（trip_walk）与三族合成攻击
（`iid_levy` / `replay_drift` / `correlated_gaussian`），完全离线、固定种子、可复现。

## 完整性校验

- `download_geolife.sh` 默认用 `unzip -t` 做结构完整性测试；
- 若你持有官方 SHA-256，可 `GEOLIFE_SHA256=<hex> tools/calibration/download_geolife.sh` 逐字节核对；
- 脚本不会自动信任未校验的镜像：请自行确认 `--url`/`GEOLIFE_URL` 来源可信。

## 合规与隐私

- **原始 GPS 不出端**：标定与线上协议一致，端上只产生 H3 cell（res10）与哈希。
  脚本与本模块都不打印坐标。
- 数据集仅在本机参与量化，标定产物（JSON）只含**聚合统计**与**量化后指标**，
  可用 `/tmp` 承载后按需删除。
- 对外发布结论时请遵守数据集许可，并说明样本来源与处理方法。
