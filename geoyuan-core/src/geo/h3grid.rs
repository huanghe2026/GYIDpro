//! H3 六边形网格 (GeoCast 地理路由核心)
//!
//! 基于 h3o crate，默认使用 Resolution 12（边长 ~9m，宽度 ~17m）
//!
//! 用于:
//! - GeoCast: 按 H3 邻近性广播 P2P 消息（非全网广播）
//! - PoL: 位置证明中的 H3 cell 定位
//! - 地理发现奖励: 首次在某 H3 cell 铸造奖励

use h3o::{CellIndex, LatLng, Resolution};

/// 默认 H3 分辨率（Res 12: ~9m 边长, ~17m 宽度）
pub const DEFAULT_H3_RESOLUTION: u8 = 12;

/// GeoCast 广播邻居环数（3 环 ≈ ~55m 半径）
pub const GEOCAST_NEIGHBOR_RINGS: u8 = 3;

/// H3 cell 包装类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct H3Cell(u64);

impl H3Cell {
    /// 从 u64 创建
    pub fn from_u64(index: u64) -> Self {
        Self(index)
    }

    /// 获取原始 u64 值
    pub fn to_u64(&self) -> u64 {
        self.0
    }

    /// 转换为 h3o CellIndex（用于计算）
    fn to_cellindex(&self) -> Option<CellIndex> {
        CellIndex::try_from(self.0).ok()
    }
}

/// 从 GPS 坐标计算 H3 cell（默认 Res 12）
pub fn gps_to_cell(lat: f64, lon: f64) -> Option<H3Cell> {
    let ll = LatLng::new(lat, lon).ok()?;
    let resolution = Resolution::try_from(DEFAULT_H3_RESOLUTION).ok()?;
    let cell = ll.to_cell(resolution);
    Some(H3Cell(cell.into()))
}

/// 从 GPS 坐标 + 自定义分辨率计算 H3 cell
pub fn gps_to_cell_with_resolution(lat: f64, lon: f64, resolution: u8) -> Option<H3Cell> {
    let ll = LatLng::new(lat, lon).ok()?;
    let res = Resolution::try_from(resolution).ok()?;
    let cell = ll.to_cell(res);
    Some(H3Cell(cell.into()))
}

/// 获取指定 cell 的 N 环邻居
pub fn get_neighbors(cell: H3Cell, ring: u8) -> Vec<H3Cell> {
    let index = match cell.to_cellindex() {
        Some(i) => i,
        None => return vec![cell],
    };

    let mut neighbors = Vec::new();
    // grid_disk_fast 返回 Option<CellIndex> 迭代器，过滤掉 None
    for distance in 0..=ring as u32 {
        for opt in index.grid_disk_fast(distance) {
            if let Some(c) = opt {
                neighbors.push(H3Cell(c.into()));
            }
        }
    }
    neighbors.sort();
    neighbors.dedup();
    neighbors
}

/// 检查两个 H3 cell 是否在同一 ring 范围内
pub fn is_within_ring(cell_a: H3Cell, cell_b: H3Cell, ring: u8) -> bool {
    let a = match cell_a.to_cellindex() {
        Some(i) => i,
        None => return false,
    };
    let b = match cell_b.to_cellindex() {
        Some(i) => i,
        None => return false,
    };
    match a.grid_distance(b) {
        Ok(distance) => distance <= ring as i32,
        Err(_) => false,
    }
}

/// 获取默认分辨率的 H3 cell 字符串表示
pub fn cell_to_string(cell: H3Cell) -> String {
    format!("{:x}", cell.0)
}

/// 从字符串解析 H3 cell
pub fn cell_from_string(s: &str) -> Option<H3Cell> {
    u64::from_str_radix(s, 16).ok().map(H3Cell)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gps_to_cell() {
        // 北京天安门坐标
        let cell = gps_to_cell(39.9042, 116.4074).expect("should produce cell");
        assert!(cell.to_u64() != 0);
    }

    #[test]
    fn test_gps_to_cell_resolution() {
        // Res 1 和 Res 12 应产生不同 cell
        let cell_r1 = gps_to_cell_with_resolution(39.9042, 116.4074, 1).unwrap();
        let cell_r12 = gps_to_cell_with_resolution(39.9042, 116.4074, 12).unwrap();
        assert_ne!(cell_r1.to_u64(), cell_r12.to_u64());
    }

    #[test]
    fn test_get_neighbors_includes_self() {
        let cell = gps_to_cell(39.9042, 116.4074).unwrap();
        let neighbors = get_neighbors(cell, 0);
        assert!(neighbors.contains(&cell));
        assert_eq!(neighbors.len(), 1);
    }

    #[test]
    fn test_get_neighbors_3_rings() {
        let cell = gps_to_cell(39.9042, 116.4074).unwrap();
        // 3 环邻居应有 37+ 个（1 + 6 + 12 + 18）
        let neighbors = get_neighbors(cell, 3);
        assert!(neighbors.len() >= 37, "3 rings should have >=37 cells, got {}", neighbors.len());
    }

    #[test]
    fn test_is_within_ring() {
        let cell = gps_to_cell(39.9042, 116.4074).unwrap();
        let neighbors = get_neighbors(cell, 2);

        // 所有 2 环邻居都应被检测为在范围内
        for n in &neighbors {
            assert!(is_within_ring(cell, *n, 2), "neighbor should be within ring 2");
        }

        // 一个远距离 cell 不应在范围内
        let far_cell = gps_to_cell_with_resolution(31.2304, 121.4737, 12).unwrap(); // 上海
        assert!(!is_within_ring(cell, far_cell, 3));
    }

    #[test]
    fn test_cell_string_roundtrip() {
        let cell = gps_to_cell(39.9042, 116.4074).unwrap();
        let s = cell_to_string(cell);
        let parsed = cell_from_string(&s).unwrap();
        assert_eq!(cell, parsed);
    }

    #[test]
    fn test_same_resolution_across_rings() {
        // 验证所有邻居在同一 resolution
        let cell = gps_to_cell(39.9042, 116.4074).unwrap();
        let neighbors = get_neighbors(cell, 2);
        let ci = cell.to_cellindex().unwrap();
        for n in &neighbors {
            let nc = n.to_cellindex().unwrap();
            assert_eq!(ci.resolution(), nc.resolution(), "all neighbors must have same resolution");
        }
    }
}
