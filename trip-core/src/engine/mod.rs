//! 临界性引擎（draft-04 §7–§8 的 Verifier 侧统计核心）。
//!
//! - [`psd`]：PSD 标度指数 α、临界置信度、H3 位移提取、收敛区间；
//! - [`levy`]：截断 Levy 分布（MLE 拟合 / 分位数 / 采样 / Levy-PSD 桥）；
//! - [`sim`]：Monte Carlo 轨迹仿真（§7.3.3 数值验证的支撑工具）；
//! - [`behavior`]：行为画像（锚点 / Markov / 节律直方图 / Levy 画像）；
//! - [`hamiltonian`]：六分量 Hamiltonian 异常评分 + 告警分级；
//! - [`trust`]：信任分 T + handle 声明门槛 + 临界封顶；
//! - [`calibration`]：GeoLife/MDC 真实数据标定（W7，人群参数校准与 ROC）；
//! - [`calibration_report`]：标定报告生成器（W7 数据白皮书草稿 Markdown）。
//!
//! 经典引擎是互操作基线：AI 增强（NeuroCriticality）只能作为独立签名的
//! 扩展证据，不得替换本模块的协议判定（GYIP-0003 §4.1）。

pub mod behavior;
pub mod calibration;
pub mod calibration_report;
pub mod hamiltonian;
pub mod levy;
pub mod psd;
pub mod sim;
pub mod trust;
