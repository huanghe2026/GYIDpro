// ─────────────────────────────────────────────────────────────────────────────
// geoyuan consensus 子命令
// ─────────────────────────────────────────────────────────────────────────────

use clap::Subcommand;
use anyhow::Result;
use colored::Colorize;

use geoyuan_core::{
    consensus::ConsensusConfig,
    consensus::validator::{Validator, ValidatorSet},
    identity::GyId,
};

#[derive(Subcommand)]
pub enum ConsensusCmd {
    /// 显示共识配置
    Config,

    /// 模拟领导者选举（测试用）
    Elect {
        /// 验证者 GyID 列表，格式: "GyIDxxx:gyid_count:stake_gy"
        /// 示例: --validators "GyIDaaa:3:1000" "GyIDbbb:5:2000"
        #[arg(long, num_args = 1..)]
        validators: Vec<String>,

        /// 模拟区块高度
        #[arg(long, default_value = "10")]
        rounds: u64,

        /// 输出为 JSON
        #[arg(long)]
        json: bool,
    },

    /// 显示法定人数计算
    Quorum {
        /// 验证者总数
        #[arg(short, long)]
        total: usize,

        /// 门槛百分比（默认 67 = 2/3+）
        #[arg(short, long, default_value = "67")]
        threshold: u8,
    },
}

impl ConsensusCmd {
    pub async fn run(self) -> Result<()> {
        match self {
            ConsensusCmd::Config => show_config(),
            ConsensusCmd::Elect { validators, rounds, json } => elect(&validators, rounds, json),
            ConsensusCmd::Quorum { total, threshold } => quorum(total, threshold),
        }
    }
}

// ─── config ──────────────────────────────────────────────────────────────────

fn show_config() -> Result<()> {
    let config = ConsensusConfig::default();

    println!("{}", "⚙  GeoYuan PoI 共识配置".bold());
    println!();
    println!("  {} {}ms", "目标出块时间:".bold(), config.block_time_ms);
    println!("  {} {}", "最小验证者数:".bold(), config.min_validators);
    println!("  {} {}%", "BFT 法定人数门槛:".bold(), config.quorum_threshold_pct);
    println!("  {} {} 块", "VRF 种子轮换:".bold(), config.vrf_seed_rotation);
    println!("  {} {} 个验证者/联邦", "联邦规模:".bold(), config.federation_size);
    println!("  {} {} GY", "最小质押:".bold(), config.min_stake / 1_000_000_000);
    println!();
    println!("{}", "投票权计算公式:".dimmed());
    println!("{}", "  voting_power = gyid_count × 1,000,000 + stake(nanoGY) ÷ 1,000,000".cyan());

    Ok(())
}

// ─── elect ───────────────────────────────────────────────────────────────────

fn elect(validator_strs: &[String], rounds: u64, json: bool) -> Result<()> {
    // 解析验证者列表
    let mut set = ValidatorSet::new();

    if validator_strs.is_empty() {
        // 默认示例
        set.add(Validator::new(
            GyId::new("GyIDnode1111111111111111111111111").unwrap_or_default(),
            5, "wx4g0e6md5".to_string(), 10_000_000_000,
        ));
        set.add(Validator::new(
            GyId::new("GyIDnode2222222222222222222222222").unwrap_or_default(),
            3, "wx4g0e6md5".to_string(), 5_000_000_000,
        ));
        set.add(Validator::new(
            GyId::new("GyIDnode3333333333333333333333333").unwrap_or_default(),
            1, "wx4g0e6md5".to_string(), 1_000_000_000,
        ));
    } else {
        for s in validator_strs {
            let parts: Vec<&str> = s.split(':').collect();
            if parts.len() != 3 {
                anyhow::bail!("验证者格式错误: '{}' (应为 GyIDxxx:gyid_count:stake_gy)", s);
            }
            let gyid_str = parts[0];
            let count: u32 = parts[1].parse().map_err(|_| anyhow::anyhow!("gyid_count 应为整数"))?;
            let stake_gy: u64 = parts[2].parse().map_err(|_| anyhow::anyhow!("stake 应为整数（GY）"))?;

            let gyid = GyId::new(gyid_str)
                .ok_or_else(|| anyhow::anyhow!("无效 GyID: {}", gyid_str))?;
            set.add(Validator::new(gyid, count, "wx4g".to_string(), stake_gy * 1_000_000_000));
        }
    }

    // 模拟选举
    use geoyuan_core::consensus::leader::LeaderElection;
    let mut election_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    let mut prev_hash = [0u8; 32];

    for height in 0..rounds {
        let seed = LeaderElection::next_seed(&prev_hash, height);
        if let Some(leader) = LeaderElection::elect(&set, height, 0, &seed) {
            *election_counts.entry(leader.id.clone()).or_insert(0) += 1;
        }
        prev_hash[0] = (height & 0xff) as u8;
        prev_hash[1] = ((height >> 8) & 0xff) as u8;
    }

    if json {
        let result = serde_json::json!({
            "rounds": rounds,
            "validators": set.all().iter().map(|v| {
                serde_json::json!({
                    "id": v.gyid.id,
                    "voting_power": v.voting_power,
                    "elected": election_counts.get(&v.gyid.id).copied().unwrap_or(0),
                })
            }).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&result)?);
    } else {
        println!("{}", format!("🗳  VRF 领导者选举模拟 ({} 轮)", rounds).bold());
        println!();
        println!("{:<36} {:>12} {:>10}", "验证者 GyID", "投票权", "当选次数");
        println!("{}", "─".repeat(62));

        let mut entries: Vec<_> = set.all().iter().map(|v| {
            (v.gyid.id.as_str(), v.voting_power, *election_counts.get(&v.gyid.id).unwrap_or(&0))
        }).collect();
        entries.sort_by_key(|b| std::cmp::Reverse(b.2));

        for (id, power, count) in entries {
            let pct = (count * 100).checked_div(rounds).unwrap_or(0);
            println!("{:<36} {:>12} {:>8} ({:>2}%)",
                id.cyan(), power.to_string().yellow(), count.to_string().green(), pct);
        }
        println!();
    }

    Ok(())
}

// ─── quorum ──────────────────────────────────────────────────────────────────

fn quorum(total: usize, threshold: u8) -> Result<()> {
    let required = (total * threshold as usize).div_ceil(100);

    println!("{}", "📊 BFT 法定人数计算".bold());
    println!();
    println!("  {} {}", "验证者总数:".bold(), total);
    println!("  {} {}%", "门槛:".bold(), threshold);
    println!("  {} {}", "需要投票数:".bold(), required.to_string().yellow().bold());
    println!();

    for n in 0..=total {
        let enough = n >= required;
        let bar_len = (n * 20).checked_div(total).unwrap_or(0);
        let bar: String = "█".repeat(bar_len) + &"░".repeat(20 - bar_len);
        let status = if enough { "✓".green() } else { "✗".red() };
        println!("  {} {}/{} [{}]", status, n, total, if enough { bar.green() } else { bar.red() });
    }

    Ok(())
}
