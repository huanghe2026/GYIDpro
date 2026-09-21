//! 标定报告生成器（W7 数据白皮书草稿）。
//!
//! 本模块将 `calibration` 模块的 JSON 输出转换为 Markdown 格式的
//! 数据白皮书草稿，用于贡献给 IETF TRIP 草案上游。

use serde_json::Value;
use std::fs;
use std::path::Path;

/// 生成数据白皮书 Markdown。
pub fn generate_whitepaper(calibration_json: &Value, output_path: &Path) -> std::io::Result<()> {
    let summary = &calibration_json["summary"];
    let pop = &calibration_json["population_report"];
    let alpha = &pop["alpha_stats"];

    let mut md = String::new();

    // 标题
    md.push_str("# GeoLife (Beijing) Population Calibration Report\n\n");
    md.push_str("*TRIP Protocol Draft-04 §7.1 α Boundary Calibration*\n\n");
    md.push_str(&format!(
        "**Generated**: {}\n\n",
        pop["metadata"]["analyzed_at"]
    ));
    md.push_str(&format!(
        "**Protocol**: {}\n\n",
        pop["metadata"]["protocol_version"]
    ));

    // 1. 执行摘要
    md.push_str("## 1. Executive Summary\n\n");
    let total = summary["total_plt_files"].as_u64().unwrap_or(0);
    let analyzed = summary["analyzed_trajectories"].as_u64().unwrap_or(0);
    md.push_str(&format!(
        "This report presents PSD scaling exponent (α) calibration results \
         from **{} trajectories** (analyzed from {} raw PLT files) \
         in the Microsoft Research GeoLife dataset (Beijing, China).\n\n",
        analyzed, total
    ));

    let bio_frac = pop["bio_fraction"].as_f64().unwrap_or(0.0);
    md.push_str(&format!(
        "**Key Finding**: {:.1}% of Chinese urban population samples fall within \
         the draft-specified biological range α ∈ [{:.2}, {:.2}].\n\n",
        bio_frac * 100.0,
        crate::engine::psd::BIO_ALPHA_MIN,
        crate::engine::psd::BIO_ALPHA_MAX
    ));

    // 2. 数据集描述
    md.push_str("## 2. Dataset Description\n\n");
    md.push_str("| Property | Value |\n");
    md.push_str("|----------|-------|\n");
    md.push_str("| Source | Microsoft Research GeoLife (Beijing) |\n");
    md.push_str(&format!("| Raw PLT files | {} |\n", total));
    md.push_str(&format!("| Analyzed trajectories | {} |\n", analyzed));
    md.push_str(&format!(
        "| H3 resolution | res{} |\n",
        pop["metadata"]["h3_resolution"].as_u64().unwrap_or(10)
    ));
    md.push_str(&format!(
        "| Min time gap | {} seconds |\n",
        pop["metadata"]["min_time_gap_seconds"]
            .as_i64()
            .unwrap_or(300)
    ));
    md.push_str(&format!(
        "| Min points/trajectory | {} |\n",
        pop["metadata"]["min_points"].as_u64().unwrap_or(64)
    ));
    md.push('\n');

    // 3. α 统计
    md.push_str("## 3. PSD Scaling Exponent (α) Statistics\n\n");
    md.push_str("### 3.1 Population Distribution\n\n");
    md.push_str("| Statistic | Value |\n");
    md.push_str("|-----------|-------|\n");
    md.push_str(&format!(
        "| N (samples) | {} |\n",
        alpha["n"].as_u64().unwrap_or(0)
    ));
    md.push_str(&format!(
        "| Mean | {:.4} |\n",
        alpha["mean"].as_f64().unwrap_or(0.0)
    ));
    md.push_str(&format!(
        "| Std Dev | {:.4} |\n",
        alpha["std"].as_f64().unwrap_or(0.0)
    ));
    md.push_str(&format!(
        "| Median | {:.4} |\n",
        alpha["median"].as_f64().unwrap_or(0.0)
    ));
    md.push_str(&format!(
        "| P5 | {:.4} |\n",
        alpha["p05"].as_f64().unwrap_or(0.0)
    ));
    md.push_str(&format!(
        "| P25 | {:.4} |\n",
        alpha["p25"].as_f64().unwrap_or(0.0)
    ));
    md.push_str(&format!(
        "| P75 | {:.4} |\n",
        alpha["p75"].as_f64().unwrap_or(0.0)
    ));
    md.push_str(&format!(
        "| P95 | {:.4} |\n",
        alpha["p95"].as_f64().unwrap_or(0.0)
    ));
    md.push_str(&format!(
        "| Min | {:.4} |\n",
        alpha["min"].as_f64().unwrap_or(0.0)
    ));
    md.push_str(&format!(
        "| Max | {:.4} |\n",
        alpha["max"].as_f64().unwrap_or(0.0)
    ));
    md.push('\n');

    // 3.2 生物区间分析
    md.push_str("### 3.2 Biological Range Analysis\n\n");
    let in_range = alpha["in_draft_bio_range"].as_f64().unwrap_or(0.0);
    md.push_str(&format!(
        "- **Draft biological range** [α_min, α_max] = [{:.2}, {:.2}]\n",
        crate::engine::psd::BIO_ALPHA_MIN,
        crate::engine::psd::BIO_ALPHA_MAX
    ));
    md.push_str(&format!(
        "- **Samples within range**: {:.1}% ({}/{}\n",
        in_range * 100.0,
        (in_range * alpha["n"].as_f64().unwrap_or(0.0)) as u64,
        alpha["n"].as_u64().unwrap_or(0)
    ));
    md.push('\n');

    // 边界推荐
    if let Some(rec) = pop["recommended_alpha_bounds"].as_object() {
        md.push_str("### 3.3 Recommended Boundary Adjustment\n\n");
        if let (Some(min), Some(max), Some(center), Some(rationale)) = (
            rec.get("recommended_min").and_then(|v| v.as_f64()),
            rec.get("recommended_max").and_then(|v| v.as_f64()),
            rec.get("recommended_center").and_then(|v| v.as_f64()),
            rec.get("rationale").and_then(|v| v.as_str()),
        ) {
            md.push_str(
                "> **Note**: The following recommendation is based on P5-P95 coverage.\n\n",
            );
            md.push_str("| Parameter | Draft | Recommended |\n");
            md.push_str("|-----------|-------|-------------|\n");
            md.push_str(&format!(
                "| α_min | {:.2} | {:.2} |\n",
                crate::engine::psd::BIO_ALPHA_MIN,
                min
            ));
            md.push_str(&format!(
                "| α_max | {:.2} | {:.2} |\n",
                crate::engine::psd::BIO_ALPHA_MAX,
                max
            ));
            md.push_str(&format!(
                "| α_center | {:.2} | {:.2} |\n",
                crate::engine::psd::ALPHA_CENTER,
                center
            ));
            md.push('\n');
            md.push_str(&format!("**Rationale**: {}\n\n", rationale));
        }
    }

    // 4. β 统计
    if let Some(beta) = pop["beta_stats"].as_object() {
        if !beta.is_empty() {
            md.push_str("## 4. Levy Stability Parameter (β) Statistics\n\n");
            md.push_str("| Statistic | Value |\n");
            md.push_str("|-----------|-------|\n");
            if let Some(n) = beta.get("n") {
                md.push_str(&format!("| N | {} |\n", n));
            }
            if let Some(m) = beta.get("mean") {
                md.push_str(&format!("| Mean | {:.4} |\n", m.as_f64().unwrap_or(0.0)));
            }
            if let Some(s) = beta.get("std") {
                md.push_str(&format!("| Std Dev | {:.4} |\n", s.as_f64().unwrap_or(0.0)));
            }
            if let Some(med) = beta.get("median") {
                md.push_str(&format!(
                    "| Median | {:.4} |\n",
                    med.as_f64().unwrap_or(0.0)
                ));
            }
            if let Some(p05) = beta.get("p05") {
                md.push_str(&format!("| P5 | {:.4} |\n", p05.as_f64().unwrap_or(0.0)));
            }
            if let Some(p95) = beta.get("p95") {
                md.push_str(&format!("| P95 | {:.4} |\n", p95.as_f64().unwrap_or(0.0)));
            }
            md.push('\n');
        }
    }

    // 5. ROC 分析
    if let Some(roc) = pop["roc"].as_object() {
        if !roc.is_empty() {
            md.push_str("## 5. ROC Analysis\n\n");
            if let Some(optimal) = roc.get("optimal_threshold").and_then(|v| v.as_f64()) {
                md.push_str(&format!(
                    "**Optimal threshold (Youden's J)**: {:.3}\n\n",
                    optimal
                ));
            }

            md.push_str("### TPR/FPR at Different Thresholds\n\n");
            md.push_str("| Threshold | TPR | FPR |\n");
            md.push_str("|-----------|-----|-----|\n");

            if let Some(tpr_at) = roc.get("tpr_at_thresholds").and_then(|v| v.as_array()) {
                for entry in tpr_at {
                    if let (Some(thresh), Some(tpr), Some(fpr)) =
                        (entry.get("threshold"), entry.get("tpr"), entry.get("fpr"))
                    {
                        md.push_str(&format!(
                            "| {:.2} | {:.3} | {:.3} |\n",
                            thresh.as_f64().unwrap_or(0.0),
                            tpr.as_f64().unwrap_or(0.0),
                            fpr.as_f64().unwrap_or(0.0)
                        ));
                    }
                }
            }
            md.push('\n');

            if let Some(auc) = roc.get("auc").and_then(|v| v.as_f64()) {
                md.push_str(&format!("**AUC**: {:.4}\n\n", auc));
            } else {
                md.push_str("*AUC requires control group data (synthetic/bot trajectories).*\n\n");
            }
        }
    }

    // 6. 方法论
    md.push_str("## 6. Methodology\n\n");
    md.push_str("### 6.1 Data Processing Pipeline\n\n");
    md.push_str("1. Parse PLT files (latitude, longitude, timestamp)\n");
    md.push_str("2. Sort by timestamp, filter intervals < 5 minutes\n");
    md.push_str("3. Quantize to H3 resolution 10 (~15,000 m²/cell)\n");
    md.push_str("4. Extract displacement sequence (haversine distances)\n");
    md.push_str("5. Compute PSD α via DFT + log-log OLS regression\n");
    md.push_str("6. Fit Levy distribution via MLE for β, κ estimation\n");
    md.push_str("7. Bridge consistency check: g = α / (3 - β)\n\n");

    md.push_str("### 6.2 Compliance with TRIP Draft-04\n\n");
    md.push_str("- DFT: Naive O(N²) definition per spec\n");
    md.push_str("- Frequency normalization: f_k = k/N\n");
    md.push_str("- Regression: Ordinary Least Squares\n");
    md.push_str("- R²: SXY²/(SXX·SYY)\n");
    md.push_str("- All floating point: IEEE-754 f64\n\n");

    // 7. 结论
    md.push_str("## 7. Conclusions & Recommendations\n\n");
    if bio_frac >= 0.90 {
        md.push_str(
            "The draft-specified α boundaries appear suitable for the Chinese urban population. ",
        );
        md.push_str("No adjustment is recommended based on this dataset.\n\n");
    } else {
        md.push_str(&format!(
            "The draft boundaries cover only {:.1}% of samples. ",
            bio_frac * 100.0
        ));
        md.push_str("Consider population-specific calibration or expanded boundaries ");
        md.push_str("for Chinese urban users.\n\n");
    }

    md.push_str("### Future Work\n\n");
    md.push_str("- [ ] Add control group: synthetic/bot trajectories for true AUC\n");
    md.push_str("- [ ] Include MDC (Lausanne) dataset for European population comparison\n");
    md.push_str("- [ ] Include T-Drive (Beijing taxi) as occupational bias reference\n");
    md.push_str("- [ ] Subgroup analysis by mobility pattern (commuter vs. tourist)\n");
    md.push_str("- [ ] Temporal drift monitoring for seasonal effects\n\n");

    // 附录
    md.push_str("---\n\n");
    md.push_str("*This report was auto-generated by trip-core calibration module (W7).*\n");
    md.push_str("*GeoYuan / TRIP Team · For IETF RATS companion publication.*\n");

    fs::write(output_path, md)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_generate_whitepaper() {
        let json = json!({
            "calibration_type": "geolife_real_data",
            "summary": {
                "total_plt_files": 100,
                "analyzed_trajectories": 85,
                "parse_errors": 5
            },
            "population_report": {
                "dataset": "GeoLife (Beijing)",
                "total_trajectories": 100,
                "analyzed_trajectories": 85,
                "alpha_stats": {
                    "n": 85,
                    "mean": 0.52,
                    "std": 0.18,
                    "median": 0.51,
                    "p05": 0.22,
                    "p25": 0.40,
                    "p75": 0.65,
                    "p95": 0.82,
                    "min": 0.15,
                    "max": 1.10,
                    "in_draft_bio_range": 0.82
                },
                "beta_stats": {
                    "n": 85,
                    "mean": 1.72,
                    "std": 0.18,
                    "median": 1.71,
                    "p05": 1.45,
                    "p95": 2.01
                },
                "bio_fraction": 0.82,
                "recommended_alpha_bounds": null,
                "roc": {
                    "tpr_at_thresholds": [
                        {"threshold": 0.0, "tpr": 1.0, "fpr": 0.18},
                        {"threshold": 0.3, "tpr": 0.88, "fpr": 0.08},
                        {"threshold": 0.55, "tpr": 0.62, "fpr": 0.02}
                    ],
                    "optimal_threshold": 0.35,
                    "auc": null
                },
                "metadata": {
                    "h3_resolution": 10,
                    "min_time_gap_seconds": 300,
                    "min_points": 64,
                    "analyzed_at": "2026-09-20T00:00:00Z",
                    "protocol_version": "draft-ayerbe-trip-protocol-04"
                }
            }
        });

        let tmp_dir = std::env::temp_dir();
        let output = tmp_dir.join("test_whitepaper.md");

        let result = generate_whitepaper(&json, &output);
        assert!(result.is_ok());

        let content = fs::read_to_string(&output).unwrap();
        assert!(content.contains("# GeoLife"));
        assert!(content.contains("Executive Summary"));
        assert!(content.contains("PSD Scaling Exponent"));
        assert!(content.contains("ROC Analysis"));

        // 清理
        let _ = fs::remove_file(output);
    }
}
