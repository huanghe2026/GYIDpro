# Synthetic (trip_walk) — Population Calibration Report

*TRIP Protocol draft-ayerbe-trip-protocol-04 §7.1 α Boundary Calibration*

**Generated**: 2026-09-24T16:42:53Z

**Protocol**: draft-ayerbe-trip-protocol-04

**Engine**: trip-core engine::calibration (same implementation as the online Verifier)

## 0. Scope & Provenance

Produced by the TRIP reference implementation (`trip-core`), using the **same** criticality engine that the online Verifier runs — so the numbers here are directly transferable to server-side decisions.

- **Positives (human)**: `trip_walk` structured-Levy generator (`engine::sim`).
- **Negatives (attack)**: three synthetic families (`iid_levy`, `replay_drift`, `correlated_gaussian`).

> ⚠️ These results characterize **generator-vs-generator separability**, i.e. a reproducibility baseline — *not* real-world performance. Re-run with real data (`gyid calibrate geolife`) before citing AUC as a real-world claim.

## 1. Executive Summary

This report presents PSD scaling exponent (α) calibration results from **150 analyzed trajectories** (input: 150 synthetic trajectories).

**Key finding**: 55.3% of samples fall within the draft-specified biological range α ∈ [0.30, 0.80].

**Discriminative power**: AUC = **0.9761** against 450 synthetic attack trajectories (3 families).

## 2. Dataset Description

| Property | Value |
|----------|-------|
| Dataset | Synthetic (trip_walk) |
| Source | trip-core engine::sim（结构化 Levy 人类 + 三族合成攻击） |
| Input (synthetic trajectories) | 150 |
| Analyzed trajectories | 150 |
| Parse errors | 0 |
| Synthetic controls | 450 |
| H3 resolution | res10 |
| Min time gap | 300 seconds |
| Min points / trajectory | 64 |

## 3. PSD Scaling Exponent (α) Statistics

### 3.1 Distribution

| Statistic | Value |
|-----------|-------|
| N (samples) | 150 |
| Mean | 0.5001 |
| Std Dev | 0.3028 |
| Median | 0.4936 |
| P5 | 0.0190 |
| P25 | 0.2791 |
| P75 | 0.7216 |
| P95 | 1.0479 |
| Min | -0.1416 |
| Max | 1.1818 |

### 3.2 Biological Range Coverage

- Draft range: **[0.30, 0.80]**, center 0.55
- Samples within range: **55.3%** (~83 of 150)

### 3.3 Recommended Boundary Adjustment

| Parameter | Draft | Recommended |
|-----------|-------|-------------|
| α_min | 0.30 | 0.0200 |
| α_max | 0.80 | 1.0500 |
| α_center | 0.55 | 0.5350 |

**Rationale**: Draft bounds [0.30, 0.80] cover only 55.3% of 150 samples. Recommended [0.02, 1.05] (P5-P95) covers 90% of population.

## 4. Levy Stability Parameter (β) Statistics

| Statistic | Value |
|-----------|-------|
| N | 150 |
| Mean | 2.8429 |
| Std Dev | 0.2616 |
| Median | 3.0000 |
| P5 | 2.2460 |
| P95 | 3.0000 |

## 5. Discriminative Power (ROC / AUC)

Positive class = real human trajectories (**150**); negative class = synthetic attack trajectories (**450**).

**AUC = 0.9761** (0.5 = chance, 1.0 = perfect)

**Optimal operating point (Youden's J)**: TPR = 0.9533, FPR = 0.1244

**Equivalent α decision band**: α ∈ [0.0119, 1.0881] (scores are `−|α − 0.55|`).

### 5.1 Control Families

| Family | N | Mean α | Median α | P5 | P95 |
|--------|---|--------|----------|----|-----|
| `correlated_gaussian` | 150 | 2.0184 | 1.8518 | 1.6939 | 2.8345 |
| `iid_levy` | 150 | -0.0141 | -0.0135 | -0.1760 | 0.1595 |
| `replay_drift` | 150 | 1.6370 | 1.6370 | 1.6369 | 1.6371 |

### 5.2 ROC Working Points

| Threshold (score) | TPR | FPR |
|-------------------|-----|-----|
| -0.0057 | 0.0067 | 0.0000 |
| -0.0453 | 0.0867 | 0.0000 |
| -0.0727 | 0.1667 | 0.0000 |
| -0.1191 | 0.2467 | 0.0000 |
| -0.1479 | 0.3267 | 0.0000 |
| -0.1794 | 0.4133 | 0.0000 |
| -0.2044 | 0.4933 | 0.0000 |
| -0.2567 | 0.5733 | 0.0000 |
| -0.3143 | 0.6333 | 0.0067 |
| -0.3600 | 0.7133 | 0.0089 |
| -0.3991 | 0.7467 | 0.0244 |
| -0.4329 | 0.8000 | 0.0333 |
| -0.4563 | 0.8400 | 0.0467 |
| -0.4775 | 0.8600 | 0.0667 |
| -0.5017 | 0.9000 | 0.0822 |
| -0.5216 | 0.9133 | 0.1044 |
| -0.5348 | 0.9467 | 0.1200 |
| -0.5448 | 0.9533 | 0.1444 |
| -0.5665 | 0.9667 | 0.1689 |
| -0.5796 | 0.9800 | 0.1911 |
| -0.5964 | 0.9800 | 0.2178 |
| -0.6175 | 0.9867 | 0.2422 |
| -0.6550 | 0.9933 | 0.2667 |
| -0.6887 | 0.9933 | 0.2956 |
| -0.7372 | 1.0000 | 0.3200 |
| -1.0869 | 1.0000 | 0.3467 |
| -1.0869 | 1.0000 | 0.3733 |
| -1.0870 | 1.0000 | 0.4022 |
| -1.0870 | 1.0000 | 0.4289 |
| -1.0870 | 1.0000 | 0.4556 |
| -1.0870 | 1.0000 | 0.4822 |
| -1.0870 | 1.0000 | 0.5089 |
| -1.0870 | 1.0000 | 0.5378 |
| -1.0870 | 1.0000 | 0.5644 |
| -1.0870 | 1.0000 | 0.5911 |
| -1.0870 | 1.0000 | 0.6178 |
| -1.0871 | 1.0000 | 0.6467 |
| -1.0912 | 1.0000 | 0.6733 |
| -1.1764 | 1.0000 | 0.7000 |
| -1.2102 | 1.0000 | 0.7267 |
| -1.2320 | 1.0000 | 0.7533 |
| -1.2485 | 1.0000 | 0.7822 |
| -1.2648 | 1.0000 | 0.8089 |
| -1.3018 | 1.0000 | 0.8356 |
| -1.3660 | 1.0000 | 0.8622 |
| -1.4486 | 1.0000 | 0.8911 |
| -1.5842 | 1.0000 | 0.9178 |
| -1.7890 | 1.0000 | 0.9444 |
| -2.1533 | 1.0000 | 0.9711 |
| -2.8278 | 1.0000 | 1.0000 |

## 6. Methodology

### 6.1 Pipeline

1. Parse input trajectories (GeoLife PLT, or `engine::sim` synthesis)
2. Sort by timestamp; drop consecutive samples < 5 minutes apart
3. Quantize to H3 resolution 10 (~15,000 m²/cell)
4. Extract displacement sequence (haversine between cell centers, km)
5. PSD α via naive DFT + log-log OLS regression
6. Truncated-Levy MLE for β, κ
7. Bridge consistency: g = α / (3 − β), expected ∈ [0.3, 0.7]
8. ROC against synthetic control families; AUC by trapezoidal integration

### 6.2 Determinism & Reproducibility

- DFT: naive O(N²) per spec; frequency f_k = k/N
- Regression: ordinary least squares; R² = SXY²/(SXX·SYY)
- All floating point is IEEE-754 f64; ROC ties are merged by score
- Control families use a fixed seed; identical seed → identical report

### 6.3 Control Families

| Family | Generator | Expected α |
|--------|-----------|------------|
| `iid_levy` | i.i.d. truncated-Levy steps (§7.3.3 literal generator) | ≈ 0 (flat) |
| `replay_drift` | replayed recording + linear drift | ≳ 1.2 (trend) |
| `correlated_gaussian` | AR(1) velocity integration | ≳ 1.0 |

## 7. Conclusions & Recommendations

The draft boundaries cover only **55.3%** of samples. Boundary recalibration should be discussed with the draft authors, with the method and raw distribution published.

Single-statistic separation is **strong** (AUC 0.9761); α alone is a useful first-line filter, though the neural evidence layer remains necessary for adversarial cases (GYIP-0003 §4.1).

### Future Work

- [ ] Add MDC (Lausanne) for cross-population comparison
- [ ] Add T-Drive (Beijing taxi) as an occupational-bias reference
- [ ] Subgroup analysis by mobility pattern (commuter vs. tourist)
- [ ] Temporal drift monitoring for seasonal effects
- [ ] Bootstrap confidence intervals for AUC and α quantiles

---

*Auto-generated by `trip-core::engine::calibration_report` (W7).*
*GeoYuan / TRIP Team · companion material for IETF RATS.*
