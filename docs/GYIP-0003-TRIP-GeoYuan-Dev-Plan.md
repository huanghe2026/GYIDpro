# GYIP-0003：GeoYuan GyID —— 全球首个 IETF TRIP 身份认证项目开发方案

> 项目代号：**TRIP-GeoYuan**（对外品牌仍为 GyID / 地理元）
> 协议基线：`draft-ayerbe-trip-protocol-04`（2026-05-08，RATS，2026-11-09 过期）
> 文档版本：v1.0 · 2026-09-16 · 作者视角：程序员 + 数据科学 + AI 工程
> 上游关系：[GNS-Foundation/trip-protocol](https://github.com/GNS-Foundation/trip-protocol)，作者在 rats@ietf.org 邮件列表公开邀请协作者
> 关联文档：[GYID2_UNIFIED_IDENTITY_PROPOSAL.md](file:///home/huanghe/myweb/GYIDpro/docs/GYID2_UNIFIED_IDENTITY_PROPOSAL.md)（本文是其 R0 闸门的具体技术落地）

---

## 0. 一页纸结论

1. **机会**：TRIP 是 2026 年 2 月才出现的 IETF 个人草案，5 个月迭代 4 版，处于 RATS 工作组视野内，**全球没有公开参考实现**；草案明确宣布传输绑定、命名系统、区块链锚定"由 companion specifications 解决"。这三个缺口恰好是 GeoYuan 已有资产（GPv1/libp2p、DID、Base 锚定）。**"首个实现 + 首批 companion 草案作者"的窗口现在开着，11 月草案过期前后是卡位黄金期。**

2. **身份重定义**：GyID = TRIP 的 **TIT**（轨迹身份令牌）+ **W3C DID 外壳**（did:geoyuan，同一把 Ed25519 密钥）+ Base 链上锚定。旧的"多因子哈希 GyID"退役为展示名。向第三方证明真人时，出示的是 Verifier 签发的 **PoH Certificate**（只含统计指数 α/β/κ/Pi，不含任何位置）。

3. **AI 是本项目相对 TRIP 原草案的代际升级，不是点缀**。草案自己承认三大未决问题：单轨迹分类的 ROC 未标定（§7.4/§13.7）、六分量不独立、机器狗攻击"仍是开放研究问题"。我们用 AI 算力正面解决：
   - **轨迹基础模型 TrajectoryFM**（自监督 Transformer）做人/合成轨迹判别、异常检测、女巫聚类；
   - **生成式红队 TRIP-Arena**（GAN/扩散模型/LLM 智能体在真实 OSM 路网上造轨迹 + 机器狗仿真）与判别器对抗共训；
   - **多模态照片面包屑**：diliy 铸卡照片经视觉模型做"场景—H3 格子—时间"一致性校验（卫星图/街景嵌入、太阳方位、天气历史）；
   - **设备端个性化小模型**（联邦学习，只上传统计量，契合 PoH 的数据最小化）；
   - **地理图灵测试**：Active Verification 时让用户完成 AI 生成的现场挑战（拍附近某物 / 限时新鲜面包屑）。
   经典统计物理引擎（PSD/Levy/Hamiltonian）**作为强制可解释基线保留**，AI 作为增强证据放进**独立签名的扩展证书**，不破坏 -04 互操作。

4. **冷启动数据优势罕见**：diliy 是线上真实产品，已有 EXIF 铸卡、GPS 网格、足迹、双人共址配对——**卡片就是语义最丰富的面包屑，共址配对就是 H_flock 的标注数据**。公开数据集用北京 **GeoLife**（182 用户）、瑞士 MDC、T-Drive 做中国人群标定（草案的 α∈[0.30,0.80] 边界明确要求按人群校准，§7.1 NOTE）。

5. **12 周出可互操作 MVP**：Rust `trip-*` crate 群（协议层零新发明，照搬 CBOR 字段）→ 经典引擎 + 1 万条 Monte Carlo → Verifier 服务 + Active Verification → GeoLife 标定与 ROC → NeuroCriticality v1 → diliy 试点。之后开源、提交 companion I-D、参加 IETF 会议周 RATS hackathon。

---

## 1. 对 TRIP-04 的实现级理解（协议契约摘要）

所有代码必须逐字段对齐，以下是不可协商的协议面：

| 机制 | 规格要点（-04） | 实现含义 |
|---|---|---|
| **面包屑 Breadcrumb** | CBOR map 9 字段：0 index / 1 Ed25519 公钥 / 2 Unix 秒 / 3 H3 cell / 4 res(7-10) / 5 context digest / 6 前块哈希(null 创世) / 7 meta flags / 8 签名；签名覆盖确定性 CBOR 编码的 0-7；块哈希=SHA256(完整 CBOR 0-8) | 需要确定性 CBOR 库（Rust `ciborium` 配 canonical 选项），任何平台同逻辑同字节 |
| **空间量化** | 出设备前必须 H3 量化（默认 res10，~15,000 m²/格），**原始 GPS 禁止进入任何协议消息** | geoyuan-core 现用 res12 做 GeoCast，需新增 res7-10 身份通道 |
| **Context digest** | `h3:` + `ts:`(5 分钟桶) + `wifi:`(BSSID 排序哈希前 16 hex) + `cell:`(基站) + `imu:`，竖线拼接 SHA-256；缺失项整体省略 | 浏览器拿不到 BSSID（隐私限制），原生 App 才能凑齐；缺失合法 |
| **链管理** | 同格连续面包屑必须拒绝；单格累计上限默认 10；采集间隔 ≥15 分钟（显式 exploration 会话可缩短，但不得 <5 分钟） | 与 diliy"地点去重 + 连续签到"逻辑天然一致 |
| **Epoch** | 默认 100 条，SHA-256 Merkle 根、左右规范序，记录 unique cell 数，身份密钥签名 | Epoch 根是上链锚定的天然对象（不要上面包屑） |
| **TIT** | Ed25519 公钥 + epoch 数 + 面包屑总数 + unique cell 数 + 信任分；CBOR/Base64url | 直接绑定 did:geoyuan，同一密钥 |
| **PSD 引擎** | 位移序列 d(i)=haversine(cell_i, cell_{i-1})，S(f)=|DFT(d)|²，log-log 拟合 S(f)∼1/f^α；窗口 64（最小）~256（推荐）；**α∈[0.30,0.80] 才判为生物**；白噪声≈0、棕噪声≈2 | 经典基线，必须逐字节复现，作为互操作检测项 |
| **临界置信度** | `confidence=(1−|α−0.55|/0.25)·R²`；<0.5 加监控，<0.3 人工复核/挑战 | 直接实现 |
| **Levy 模型** | P(Δr)∼Δr^−β·e^(−Δr/κ)，β 典型 1.50–1.90；每 epoch MLE 拟合；超 99.9 分位记异常；Levy-PSD 桥 α_eff≈(3−β)·g，g≈0.4–0.6；β 与 α 必须做一致性校验 | 数据科学模块 |
| **行为画像** | anchor cell（≥5 条）、Markov 转移矩阵（每 epoch 重建）、Pi 可预测性人类 0.80–0.95、24 小时昼夜 + 7 天周节律直方图 | 设备端/Verifier 双侧可算 |
| **六分量 Hamiltonian** | H=0.25H_spatial+0.20H_temporal+0.20H_kinetic+0.15H_flock+0.10H_contextual+0.10H_structure；权重乘成熟度 m=min(n/200,1)；告警 NOMINAL/ELEVATED(<3)/SUSPICIOUS(<5)/CRITICAL | H_flock 取最近 k=7 个共址实体速度向量；缺失时回退本人同时同地历史分布 |
| **PoH 证书** | CBOR 15 字段：公钥/签发时间/epoch 数/α/β/κ/Pi/置信度/T/unique cell/总数/有效期/**RP nonce(必选)**/**链头哈希(必选)**/Verifier 签名。无任何原始位置 | Attestation Result；我们额外封装为 W3C VC |
| **信任分** | T(%)=40·min(n/200,1)+30·min(unique/50,1)+20·min(days/365,1)+10·chain_integrity；claim handle 需 n≥100 且 T≥20；临界测试失败 T 强制封顶 50 | handle = geoyuan.com @昵称 |
| **Active Verification（唯一模式）** | RP 生成 16 字节 nonce → Verifier 实时挑战（WS/推送）→ Attester 签 nonce+chain head+index 响应 → Verifier 校验后出绑定 nonce 的 PoH | passport 模式不禁止但更弱；我们两种都支持，主推 active |
| **收敛区间** | <64 bootstrap 不算 PSD；64–199 provisional（α 方差~0.15）；200+ stable（方差<0.05）；64 初筛 / 100 可 claim handle / 200 可靠正判 / 256+ 高风险决策 | 产品等级 L0–L4 直接映射 |
| **多 Verifier** | 任何实现合规者皆可当 Verifier，各持 Ed25519 密钥；Attester 可提交多个；RP 自选信任 | **我们的定位：第一个公开 Verifier，不是唯一 Verifier** |
| **低移动性** | 禁止设最低空间多样性；静止用户以"时间 PSD"补充；时间连续性+条数占 60% 权重 | 残障/居家用户合规，必须测 |

**草案明确的开放问题（=我们的论文/标准贡献点）**：
- 单轨迹应用集成统计量的 ROC/置信区间未经验证（点名需用 GeoLife/MDC 做 companion publication）；
- Monte Carlo 规范已写（10,000 条截断 Levy，β∈[1.50,1.90]，外加随机游走/重放/高斯相关游走对照组）但无人实现；
- α 边界需人群标定；
- 六分量非独立，组合错误率需实测；
- 机器狗/无人机绑机攻击"active area of research"。

---

## 2. 为什么 GeoYuan 是 TRIP 的"天选首实现"

### 2.1 资产映射（协议要求 → 现成代码）

| TRIP 要求 | 现成资产 | 差距 |
|---|---|---|
| 身份 Ed25519 密钥 | [geoyuan-core/src/crypto/signer.rs](file:///home/huanghe/myweb/GYIDpro/geoyuan-core/src/crypto/signer.rs) | 几乎零差距 |
| H3 量化与地理运算 | geoyuan-core 已引 H3 | res12→res10 默认值 |
| 签名的位置存在声明 | GPv1 PoL（H3+时间戳+Ed25519） | 改 CBOR 字段与哈希链格式 |
| 追加式轨迹日志 | redb 本地存储 + P2P 同步 | 加 epoch/Merkle |
| 传输绑定（草案留白） | GPv1 libp2p GeoCast | **写 companion I-D** |
| 区块链锚定（草案留白） | diliy Base ERC-721 + anchor-worker + payload_hash | 新增 epoch 根锚定合约 |
| 命名系统（草案留白） | gyid.geoyuan.com 品牌、WebButton | did:geoyuan + handle |
| H_flock 共址群体 | diliy 卡片配对（双人同事件）、geo-proxy | 共址会签面包屑 |
| Context 照片/IMU | diliy EXIF 铸卡（照片哈希+GPS+时间） | 多模态扩展字段 |
| RP 付费验证（远期） | diliy x402 支付试点 | 计量 API |
| 跨端 Attester | gyid-sdk 已有 WASM/Android/iOS/C 目标 | 面包屑采集器 |
| 行为画像冷数据 | footprint 足迹、卡片网格（~1.1 亿 H3 key 经验） | 导出训练样本 |

### 2.2 产品级判断：面包屑从哪来（最关键的产品问题）

TRIP 要求 100–200 条面包屑才进入可靠区间，15 分钟间隔意味着需要**数十小时持续移动**。纯后台常驻采集在 iOS/Android 上耗电、审核敏感、用户戒心强。GeoYuan 的破局点是**分层自愿采集**：

| 面包屑源 | 场景 | 间隔/数量 | 丰富度 | 端 |
|---|---|---|---|---|
| **卡片面包屑**（主力） | 用户在地理元铸卡，照片 EXIF 天然含 GPS/时间 | 每张卡 1 条，可批量补传 | ★★★★★（照片多模态） | Web/移动 Web 即可 |
| **足迹打卡** | 现有 footprint 签到 | 按地点去重 | ★★★ | Web |
| **探索会话** | App 内"开启轨迹"的步行/旅行模式（草案许可的 exploration） | 5–15 分钟 | ★★★★ | 需原生 |
| **共址会签** | 双人见面扫卡/配对，互签同一 H3+时间桶 | 事件触发 | ★★★★（flock 真值标签） | Web+推送可做 |
| **后台被动**（可选高级） | 高信任用户 opt-in | ≥15 分钟 | ★★★★ | 原生 App |

含义：**第一阶段只靠铸卡/打卡就能在 Web 上跑通协议骨架**（卡片稀疏→bootstrap 等级，诚实展示"成长中的 GyID"）；原生 App 解决 200+ 条的稳定区间。这也完美契合"GyID 随生活积累变可信"的叙事。

---

## 3. 系统总体架构

```
                         ┌──────────────────── Relying Party ────────────────────┐
                         │  geoyuan.com(本人) │ 第三方网站/AI Agent │ 企业风控      │
                         └───────────┬───────────────────┬───────────────────────┘
              VerificationRequest(nonce)          PoH Certificate（只含统计指数）
                         ▼                          ▲
┌──────────────────── Verifier（geo-verify 服务，GPU 池）─────────────────────────┐
│ 接入网关  WebSocket 实时挑战 / 速率限制 / RP 注册表（远期 x402 计量）             │
│ 链验证器  index 连续 · 时间单调 · prevHash · Ed25519 · 去重/间隔规则            │
│ 经典 Criticality Engine（强制、可解释、互操作基线）                              │
│   PSD(DFT,α) · Levy MLE(β,κ,桥一致性) · Markov/Pi · 昼夜周节律 · 6 分量 H      │
│ NeuroCriticality Engine（AI 增强，独立扩展证据）                                 │
│   TrajectoryFM 嵌入 · 人/合成分类 · 异常分 · 女巫聚类 · 多模态照片一致性         │
│ 证书签发  标准 PoH CBOR(15 字段,Verifier 密钥签) + GeoYuan 扩展(单独签)         │
│ 标定与红队  人群参数库 · TRIP-Arena 生成器 · ROC/漂移监控（离线）                │
└───────▲───────────────────────────┬───────────────────────────────────────────┘
  H3 Evidence(CBOR)          epoch roots / TIT 注册 / 挑战推送
        │                           │
┌───────┴──────── Attester 端 ──────┴──────────────────────────────────────────┐
│ trip-attester（Rust core → WASM / iOS / Android）                              │
│  密钥保管(Ed25519+助记词/设备树) · 原始GPS不出端 · H3 res10 量化                │
│  context digest(wifi/cell/imu/photo) · 签名链 · epoch 密封 · redb 本地日志     │
│  设备端个性化小模型(昼夜/Markov/微型GRU) · Active 挑战响应                       │
│ 采集面：diliy 铸卡页 / 足迹 / 探索会话(原生) / 共址会签                          │
└───────┬───────────────────────────────────────────────────────────────────────┘
        │ 锚定（只锚哈希，不锚轨迹）
        ▼
┌── Base 主网 ─────────────────┐   ┌── IPFS ──────────┐   ┌── GPv1 libp2p ──────┐
│ GeoTITRegistry(TIT→DID/      │   │ 照片/凭证加密包   │   │ 多 Verifier/节点间  │
│ handle/epoch root 序列)      │   │ 资产导出包        │   │ Evidence 中继/备份  │
└──────────────────────────────┘   └──────────────────┘   └─────────────────────┘
```

三条不可违背的架构原则：

1. **原始 GPS 永不出端**（草案强制 + PIPL）。端上只产 H3 cell 与哈希；Verifier 落库的也是量化数据，保留期限明示，RP 永远只见统计指数。
2. **经典引擎是合规基线，AI 是增强证据**。扩展结论放在独立的 `GeoYuan-Extension` CBOR 容器里（内含标准 PoH 证书 + 神经分 + 多模态分 + Verifier 签名），只懂 -04 的 RP 仍可验证标准字段。
3. **链上只锚存在性**：创世公钥、epoch Merkle 根、TIT-handle 绑定、证书吊销位。不锚 cell、不锚照片原图。

---

## 4. 创新设计：AI 原生的轨迹身份（七个创新点）

> 设计纪律：每个 AI 创新都必须对应草案中的一个**已承认缺陷**或一个**字段扩展位**，不做悬浮的"AI 概念"。

### 4.1 NeuroCriticality：轨迹基础模型（对应 §13.7 开放问题）

**动机**：六分量是手工特征 + 线性加权，草案承认分量不独立、组合错误率未测、机器人攻击无解。

**模型**：
- **TrajectoryFM v1（判别）**：输入量化轨迹序列（cell token、时间桶、位移、context flag 集合），结构为 4–6 层小型 Transformer（~10–30M 参数），自监督预训练任务：
  1. masked cell 恢复（随机掩 15% cell，预测邻居分布）；
  2. 下一格/下一时间桶预测；
  3. 轨迹片段乱序判别（对比学习）；
  4. 人 vs 合成轨迹二分类微调（标签来自公开真人数据集 + TRIP-Arena 合成器）。
- 三个输出头：`p_human`、逐面包屑异常分（替代/校准 Hamiltonian）、轨迹嵌入向量（128 维）。
- **与经典引擎的集成方式**：神经分不覆盖 α 判定，而是
  - 一致 → 提高置信度（缩短达成同等级所需面包屑数的贝叶斯更新）；
  - 不一致 → 触发 Active Verification 加挑战 + 人工复核；
  - 嵌入向量做**跨身份女巫聚类**（同一合成器造的轨迹在嵌入空间成簇，这是手工特征做不到的）。
- 边缘蒸馏：蒸馏 <2MB 的小模型进 Attester，做本地粗检与隐私友好的个性化。

**可解释性**：对每个判定输出归因（哪些片段/分量贡献），避免黑箱风控伤用户；对外只暴露分数等级，不泄模型细节。

### 4.2 TRIP-Arena：生成式对抗红队（用 AI 算力换安全性）

草案 §7.3.3 要求 Monte Carlo 对照组，我们升级为**全谱系攻击生成器 + 自动化评测场**：

| 攻击族（草案点名） | 生成器 |
|---|---|
| 纯随机游走 / 高斯相关游走 | 统计过程基线 |
| 截断 Levy 仿真（β 抽样） | 草案规范实现，10,000 条起 |
| 轨迹重放 + 漂移 | 真人轨迹切片 + 时间扭曲/空间平移 |
| POI 感知脚本轨迹 | 在真实 OSM POI/路网图上跑 Markov（"懂城市的 bot"） |
| **TrajGAN / 扩散轨迹模型** | 学习真人轨迹分布生成（主攻 α 伪造） |
| **LLM Agent 轨迹** | LLM 调用地图/天气/日历工具按"人生脚本"逐日安排移动（最难的语义级伪造） |
| **机器狗/无人机绑机** | 用四足机器人与无人机公开 IMU/GPS 运动谱仿真，攻击 PSD 1/f 与相图平滑性 |
| 模拟器注入 | 无 wifi/cell/IMU context 的降级轨迹 |

训练循环：生成器以"骗过当前引擎"为奖励（RL/进化策略），判别器迭代；**每月冻结一次 Arena 版本生成 leaderboard 与 ROC 曲线**，成为论文与标准贡献的素材，也成为第三方 RP 选 Verifier 的客观依据。产出开源，直接回应草案"需要 companion publication"。

### 4.3 多模态照片面包屑（GeoYuan 相对所有 TRIP 实现的独占差异）

diliy 每张卡 = 照片 + EXIF(GPS, 拍摄时间) + 文字。把照片哈希纳入 context digest 的 `photo:` 扩展，并在 Verifier 侧做**五重一致性校验**（只在用户提交验证时按需触发，算力可计费）：

1. **EXIF 篡改检测**：字段一致性模型（厂商 maker note、软件痕迹、缩略图与主图差异、GPS 时间与系统时间差）；
2. **视觉地点识别（VPR）**：照片 CLIP 类嵌入 与 声称 H3 cell 的卫星图/街景瓦片嵌入做相似度（中国境内用合规地图影像服务）；
3. **太阳几何校验**（确定性、零成本）：拍摄时刻太阳方位/高度 vs 照片光照与阴影方向；
4. **天气历史校验**：第三方历史天气 API（晴雨与画面）；
5. **路网/移动方式匹配**：相邻面包屑速度在 OSM 交通方式合理域内。

输出一个 `photo_consistency ∈ [0,1]` 进扩展证据。**隐私设计**：默认仅传照片哈希；视觉校验可在端上（端侧小模型）或用户授权后传加密图，原图不进 Verifier 长期库。

### 4.4 地理图灵测试（Active Verification 的 GeoYuan 增强）

标准 Active 只验"密钥持有 + 链头新鲜"。增强为三级挑战，按风险分级触发：

- **L1 标准**：签 nonce（秒级，草案原样）；
- **L2 新鲜行走**：15–60 分钟内产生 ≥1 条满足方向/格子约束的新面包屑（防静态重放）；
- **L3 现场多模态**：AI 根据所在 H3 实时生成**位置绑定的拍摄挑战**（"拍一张你视野里的公交站牌/某类店面"），VLM 判定 + 太阳/地点一致性。机器狗能扛手机走路，但完成语义拍摄挑战的边际成本极高；虚拟定位则过不了视觉地点识别。

### 4.5 设备端联邦个性化（满足 §8 学习画像且不上传轨迹）

昼夜直方图、anchor Markov、κ 这些"个人基线"天然可端上计算。进一步：
- 端上微型 GRU（<1MB）做本人异常检测，只把"异常标记 + 统计量"随 Evidence 上报；
- 全局模型改进以**联邦学习/安全聚合**方式进行（按周聚合梯度，不留单用户轨迹）；
- 无 IMU 权限时按草案 §9.5 等比提升其余分量权重——由端上声明能力位，Verifier 防止"谎称无传感器套利"（长期无 contextual 的身份信任增长限速）。

### 4.6 Flock 真值网络：把 diliy 社交变成抗女巫基础设施

草案 H_flock 依赖"共址 TRIP 实体"，冷启动没有 flock。diliy 的双人配对/同事件卡片是**带标签的共址数据**：
- 会签面包屑：两密钥在同一 5 分钟桶、同 H3 cell 互签（实现草案 flock 的离散版），并成为模型训练的正样本；
- 图算法：在"共址图 + 轨迹嵌入"上跑社团/异常簇检测，识别批量养号（一群身份轨迹嵌入成簇且从无独立社交连接）；
- 信任传递有阻尼：共址只加"同时在场"佐证，不传递真人结论，防止互刷。

### 4.7 PoH 即登录：面向 AI Agent 时代的 Relying Party 产品

2026 年 RP 最大痛点是区分"真人用户 vs AI Agent/批量号"（World ID 的全部叙事）。我们提供：
- **Sign-In with GyID（SIWG）**：OAuth 风格按钮 → 后台跑 Active Verification → RP 收到标准 PoH（α/置信度/T）+ 扩展等级，不接触位置；
- 分级策略示例：n<64 仅浏览；T≥20(n≥100) 可发言/领 @handle；T≥60 + 神经分通过 可领福利/投票；高风险交易走 L3 挑战；
- **计量**：复用 diliy x402 经验，Verifier API 对商业 RP 按次计费（独立于 GYU，境外结算主体单设），形成"AI 算力成本 → 验证收入"闭环；
- PoH 封装为 **W3C Verifiable Credential**（`credentialSubject` 只放统计指数），未来任何 SSI 钱包可收。

---

## 5. 工程分解

### 5.1 Rust workspace（新增 crate，复用 geoyuan-core）

```
GYIDpro/
├─ geoyuan-core/        # 既有：crypto/h3/redb/p2p —— 改造为共享底座
├─ trip-core/           # 【新】纯协议，no_std 友好，零网络依赖
├─ trip-attester/       # 【新】采集/量化/context/签名链/epoch/本地库
├─ trip-verifier/       # 【新】链校验 + 经典 Criticality Engine
├─ trip-neuro/          # 【新】ONNX 运行时绑定 + 特征桥（Rust 侧推理）
├─ trip-server/         # 【新】Verifier 服务：HTTP/WS、RP、挑战、证书签发
├─ trip-cli/            # 【新】trip breadcrumb/epoch/verify/poh/new ...
└─ gyid-sdk/            # 既有 FFI：WASM/Android/iOS 暴露 attester API
```

**trip-core 关键模块与 API（第一行代码就按此切）**：

```rust
// 严格对齐草案 Table 2
pub struct Breadcrumb { pub index:u64, pub identity:PublicKey32, pub ts_unix:u64,
    pub h3_cell:u64, pub h3_res:u8, pub ctx_digest:[u8;32],
    pub prev_hash:Option<[u8;32]>, pub meta:MetaFlags, pub sig:[u8;64> }

impl Breadcrumb {
    pub fn signable_cbor(&self) -> Vec<u8>;      // 确定性 CBOR 字段0-7
    pub fn sign(&mut self, sk:&SecretKey);       // Ed25519
    pub fn block_hash(&self) -> [u8;32];         // SHA256(CBOR 0-8)
}
pub struct Epoch { number, identity, first_idx, last_idx, t0, t1,
                   merkle_root:[u8;32], unique_cells:u32, sig:[u8;64] }
pub struct PohCertificate { identity, iat, epochs, alpha, beta, kappa, pi,
    confidence, trust, unique_cells, count, validity, nonce:[u8;16],
    chain_head:[u8;32], verifier_sig:[u8;64] }   // 15 字段，一个不少
pub mod engine { pub fn psd_alpha(window:&[Displacement]) -> (f64,f64/*R2*/);
                 pub fn levy_fit(...) -> (Beta,Kappa);
                 pub fn hamiltonian(...) -> HReport; /* 6 分量+权重+m */
                 pub fn trust(n,unique_cells,days,chain_ok,criticality_ok)->f64; }
```

- 确定性 CBOR：`ciborium` 不够的地方自管 map key 升序；配套**黄金测试向量**（固定密钥/固定轨迹 → 固定字节的 CBOR、哈希、签名），作为未来其他语言实现的互操作基准（这是"参考实现"地位的核心资产）。
- 依赖选型：`ed25519-dalek`、`h3o`（纯 Rust H3，免 C 构建，利于 WASM/移动端）、`sha2`、`redb`、`merkle-cbt` 或自写 12 行 Merkle、`ort`（ONNX Runtime 绑定，仅 verifier/neuro feature）。

### 5.2 Verifier 服务（trip-server）

- 无状态计算层 + Postgres（Evidence 量化数据，分区保留策略）+ Redis（nonce/挑战）+ WebSocket 推送；
- 经典引擎 Rust 直算；神经推理走独立 GPU worker（ONNX/Triton），CPU 降级；
- Verifier 自己一把 Ed25519 长密钥，公钥多渠道公布（官网、IPFS、未来 did:web）；
- 接口：
  - `POST /v1/evidence`（Attester 批量上传面包屑/epoch，端上加密传输）；
  - `POST /v1/verify`（RP 发起，返回 challenge_id）；
  - `WS /v1/challenge`（Attester 接挑战、回 LivenessResponse）；
  - `POST /v1/poh`（RP 轮取/回调，验签后得证书）；
  - `GET /.well-known/verifier.json`（公钥、保留策略、支持的扩展、Arena 成绩）。

### 5.3 Base 合约（不发币、平台代付 gas）

`GeoTITRegistry.sol`（可升级性最小化，优先不可变）：
- `register(pubkey32, did_suffix)` 首次绑定；
- `anchorEpoch(pubkey32, epoch_no, merkle_root, unique_cells)`（由 Verifier 或提交者调，只存根+时间；同 epoch 重复提交即告警）；
- `claimHandle(pubkey32, name)` 需链下签名证明 n≥100/T≥20（Verifier 出短证）；
- PoH 不上链（一次性、含 nonce）；证书状态可走吊销注册表（远期）。
- 与 [DiliyCard.sol](file:///home/huanghe/myweb/diliy/contracts/DiliyCard.sol) 解耦：卡片 mint 事件可被 indexer 关联为"卡片面包屑"的存在性佐证。

### 5.4 did:geoyuan 与证书格式

- DID = `did:geoyuan:z<base58(Ed25519 pubkey)>`；DID Document 含 verificationMethod、TIT service endpoint、GPv1 libp2p 端点、Base anchor 指针；
- 展示名：@handle（geoyuan.com 昵称，100 面包屑门槛）；旧 GyID 字符串做一次性迁移映射；
- PoH-VC：标准 JWT-VC/SD-JWT 封装 PoH CBOR，支持选择性披露（只给 T 不给 α 等）。

### 5.5 diliy（geoyuan.com）改造点

1. cards 迁移：`owner_did / owner_sig / trip_index / epoch_no / photo_ctx`（同 GYID2 方案，合并实施，不做两次迁移）；
2. 铸卡页加载 WASM attester：本地量化 EXIF→H3、构造面包屑、入本地链；上传时一并提交；
3. anchor-worker：卡片上链后调 trip-server `/evidence`，epoch 满触发 `anchorEpoch`；
4. 新增 **GyID 成长页**：TIT 公钥、面包屑数/unique cell/连续天数、T 分、α（可选展示）、等级 L0–L4、当前链头、Base/IPFS 链接、助记词备份强引导；
5. ProofSeal 升级：展示"TRIP 真人轨迹证明"等级与最近一次 PoH 验签；
6. 配对页加"共址会签"；
7. SIWG 按钮组件（收编 WebButton）供第三方站点嵌入；
8. 原生 App：Capacitor 包现有前端 + 原生插件（位置后台、Wi-Fi 扫描、IMU、推送）；iOS 后台模式以"旅行/探索"显式会话过审。

---

## 6. 数据科学计划

| 任务 | 数据/方法 | 产出 |
|---|---|---|
| 协议 Monte Carlo（§7.3.3 硬性 SHOULD） | 10,000 条截断 Levy（β∼U[1.5,1.9]、κ 对数正态），量化 res10 + 去重；对照组：随机游走、真人轨迹重放、高斯相关游走 | 验证 g∈[0.3,0.7]、α 落点分布 |
| 中国人群标定 | **GeoLife（北京 182 人/17k+ 轨迹）**、T-Drive（北京出租车，注意职业偏差仅作对照）、MDC（洛桑 200 人） | 实测 α/β/κ/Pi 分布，给出中国人口校准边界（保留 0.55 中心原则） |
| ROC 与收敛曲线 | 在 64/100/200/256 四个截点算 FPR/FNR、置信区间 bootstrap | 回应 §7.4，内部等级阈值与对外白皮书 |
| 分量相关性 | 六分量两两相关/条件互信息，重新拟合有效组合权重（广义加性/逻辑回归，可解释） | 对草案的改进证据 |
| 低移动性专项 | 居家/残障合成子集 + 静止真人公开数据；时间 PSD 补充特征 | 确保不歧视（§15.5 强制） |
| 概念漂移监控 | 线上分数分布月度监控、Arena 每月回归 | Verifier 版本化与重标定 SOP |
| 隐私审计 | H3 res 与再标识风险（结合人口密度，§14.5）、k-匿名式粗化策略 | 农村自动降 res、用户覆盖开关 |

实验跟踪：MLflow/DVC，全部随机种子固定；协议经典引擎的测试结果进 CI 做数值回归（α 误差 <1e-6）。

## 7. AI 算力计划（务实预算）

| 阶段 | 算力形态 | 量级估算 | 用途 |
|---|---|---|---|
| 经典引擎/Monte Carlo | CPU 即可 | 单机数小时 | Rust 多线程，无需 GPU |
| TrajectoryFM 预训练 | 单卡 A100/H100 租赁或国产同算力云 | 10–30M 模型 × 百万级轨迹，约 2–5 卡日 | 自监督 + 分类微调 |
| 红队扩散/GAN 训练 | 同上 | 与判别器交替，约 3–7 卡日/轮 | Arena v1 |
| 多模态 VPR | API 优先（合规影像/多模态大模型），端侧小模型备选 | 按调用量 | 照片一致性，先不自建大模型 |
| 线上推理 | Verifier GPU worker（Triton/ONNX，支持 CPU 降级） | 单次验证 <100ms，单卡可承载数千次/日 | PoH 签发 |
| 端侧 | 手机 NPE/CoreML 蒸馏模型 | <2MB、<50ms | 本地粗检/个性化 |

原则：**不追大模型，追数据闭环与对抗积累**。最贵的多模态调用只在 L3 挑战触发；预算优先给 Arena（攻击迭代）而非模型堆参数量。

---

## 8. 路线图（12 周 MVP + 6 个月标准/产品化）

| 周次 | 里程碑 | 验收标准 |
|---|---|---|
| W1 | trip-core：CBOR/签名/哈希链/epoch | 黄金测试向量；与草案字段 100% 对齐 |
| W2 | 链规则（去重/间隔/验证）+ trip-cli | 命令行造链、验链全绿 |
| W3 | 经典引擎：PSD/Levy/Markov/节律 | 合成数据上 α 误差达标；Monte Carlo 1 万条跑通 |
| W4 | Hamiltonian 6 分量 + 信任分 + PoH 签发/验签 | 单进程端到端 RP 模拟通过 |
| W5 | trip-server：evidence/verify/WS 挑战 | Active Verification 三方流程联机 |
| W6 | WASM attester + 演示页（手动上传 GPX/照片轨迹） | 浏览器产 CBOR，服务端验证一致 |
| W7 | GeoLife/MDC/T-Drive 标定 + ROC 报告 | 首份数据白皮书草稿（贡献上游素材） |
| W8 | Base GeoTITRegistry 测试网 + did:geoyuan 解析 | epoch 根可在 basescan 查；DID Document 可解析 |
| W9 | NeuroCriticality v1 训练（FM+分类头） | Arena 基线 FPR/FNR 优于纯经典引擎 |
| W10 | TRIP-Arena v1（6 类攻击器）+ 扩展证书容器 | 月度 leaderboard 自动出榜 |
| W11 | diliy 内测：铸卡即面包屑 + GyID 成长页（10–50 内测用户） | 真实用户产生首条链上 TIT |
| W12 | 主网锚定 + PoH-VC + SIWG demo 站 + 开源发布 | **"全球首个 TRIP 身份实现"公开发布** |
| M4 | 共址会签、足迹接入、照片五重校验 v1、L2 挑战 | 内测扩量 |
| M5 | 原生 App（Capacitor+插件）探索会话 → 用户可冲 200+ 条 | 首批 stable 等级 GyID |
| M6 | 标准化战役（见 §9）+ 商业 RP 试点 + x402 计量 | IETF 反馈/companion draft 受理 |

里程碑门：**W4 不通就不做 AI**——先证明协议层零偏差，AI 才有附着点。

---

## 9. 标准化战略：从"实现者"变成"TRIP 生态共同作者"

草案作者公开邀请协作（rats 邮件列表原话："Feel free to fork and submit, or I can add you as a collaborator"）。这是个人/小团队极少有的标准卡位机会。

1. **立刻（W1–W2）**：
   - 向 rats@ietf.org 发一封实现意向邮件（参照协议措辞，汇报我们在做 Rust 互操作实现 + 黄金测试向量计划）；
   - fork 上游，开始提文档级 PR（错别字/歧义先混脸熟），star/watch，跟进 -05；
2. **三份 companion I-D（W6 起草，M6 提交）**，正好填草案 §15.3/§15.4 和正文留白：
   - `draft-huanghe-trip-transport-libp2p`：基于 GPv1 的 Evidence 中继、多 Verifier、实时挑战通道绑定；
   - `draft-huanghe-trip-blockchain-anchor`：epoch 根/TIT 注册的 EVM 锚定与撤销模型；
   - `draft-huanghe-trip-did-binding`：TIT 与 W3C DID/VC/handle 的绑定规范（SIWG）；
   - （可选第四份）`draft-huanghe-trip-neural-evidence-extension`：AI 增强证据的容器格式与隐私要求——把 §4 的创新标准化，而不是只做产品；
3. **证据公开**：黄金测试向量、Monte Carlo 与 GeoLife 标定结果（数据合规前提下）、Arena 开源，主动承担草案点名"需要 companion publication"的实证工作，争取学术联合作者；
4. 参加 **2026 年 11 月 IETF 会议周 RATS 议程与 Hackathon**（草案 11 月 9 日过期，正是 -05/换工作组讨论窗口），目标：hackathon 做现场互操作演示，bar BoF 提案"轨迹证明"后续工作；
5. 品牌话术克制：对外称"**the first public open-source implementation of IETF TRIP draft-04**"（可验证：开源仓库时间戳、IETF 邮件归档），不说"IETF 认证"（个人草案无 IETF 背书，合规红线）。

---

## 10. 风险清单（工程/科学/合规/战略）

| 风险 | 等级 | 对策 |
|---|---|---|
| 草案大改/废弃/被工作组拒绝 | 高 | 代码协议层模块化（可换版本）；pin -04；即使 TRIP 失败，DID+多模态凭证架构独立成立（GYID2 兜底） |
| α 等手工阈值对中国人群不适用 | 中高 | GeoLife/T-Drive 标定（W7），按草案授权调整边界并公开方法；AI 头兜底 |
| AI 误判真人（假阴性伤害用户） | 中高 | 错误成本不对称设计（草案 §7.4.3）：假阴性仅降速成长不封号；人工申诉；经典+神经双轨一致才处罚 |
| 对抗样本绕过神经模型 | 高 | Arena 持续迭代；神经分永远不单独作为拒绝依据；L3 多模态挑战 |
| 机器狗攻击 | 中 | 诚实承认开放问题；语义拍摄挑战 + 相图/IMU 谱 + 节律；写入白皮书不吹牛 |
| 常驻定位的隐私/耗电/审核 | 高 | 默认零后台；卡片/打卡优先；原生探索会话显式开关；原始 GPS 不出端；PIPL 告知同意、保留期限、删除权（Verifier 必须支持删除，§14.2） |
| H3 res10 在农村可识别 | 中 | 按人口密度自动降 res + 用户手动降级（§14.5） |
| Verifier 中心化争议 | 中 | 自我定位为多 Verifier 之一；公开公钥/策略/Arena 成绩；GPv1 支持证据多投；鼓励第二方实现并提供测试向量 |
| 平台代持/关站风险 | 中 | DID 私钥用户自持 + Base 锚定 + IPFS 导出包（承接 GYID2 §3.4） |
| 合规（区块链备案、地图资质、位置数据出境） | 高 | 境内 Verifier 与地图数据境内闭环；多模态影像用合规底图；x402/商业验证境外主体隔离；不发币、GYU 不混用 |
| "首个"被抢跑 | 中 | 速度即护城河：W12 公开 + IETF 邮件列表留痕 + 测试向量与 Arena 的持续输出比口号有效 |
| 数据冷启动慢（200 条门槛） | 中 | 卡片密集场景（校园用户批量旅行卡）+ 探索会话；bootstrap 期诚实展示"成长中"，不提前发高等级 |

---

## 11. 立刻可做的第一步（本周）

1. 建分支与四个 crate 骨架，落 `Breadcrumb/Epoch/PohCertificate` 的 CBOR 结构与黄金向量测试（1–2 天）；
2. 把 -04 的 Monte Carlo 对照组先实现（纯 CPU、纯统计，是后续一切的标尺）；
3. 发出 rats@ietf.org 实现意向邮件 + fork 上游；
4. 下载 GeoLife/MDC 做数据获取与授权核查；
5. 与 diliy 合并数据库迁移设计（owner_did 一套字段服务两个方案）。

---

## 12. 决策清单（需要你拍板）

1. 是否以 **TRIP-04 为 GyID 2.0 的协议基线**（GyID 本质 = TIT+DID，旧哈希退役）？
2. 是否同意 **W4 协议层门、W12 公开发布** 的节奏，以及先 Web（卡片面包屑）后原生 App 的采集路线？
3. AI 投入是否按"**经典引擎强制 + 神经增强 + Arena 红队**"双轨，而非直接上黑箱模型？
4. 是否启动 **IETF companion drafts + 与上游作者联系**的标准化路线（可能涉及以个人/公司名义署名）？
5. 算力预算：先按**租赁单卡 A100 级、总量十数卡日**起步是否可接受？

---

*本文档为开发行动方案；协议引文全部来自 draft-ayerbe-trip-protocol-04 原文（已逐节核对）。下一步可据此拆 W1 的 issue 与测试向量规范文档。*
