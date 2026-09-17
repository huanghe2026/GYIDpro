//! H3 六边形网格模块
//!
//! 提供基于 Uber H3 的地理空间网格系统集成。
//! H3 将地球表面划分为分层六边形网格，支持 16 个分辨率级别。

use h3o::{CellIndex, LatLng, Resolution};
use serde::{Deserialize, Serialize};

/// H3 单元格信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct H3Cell {
    /// H3 单元格索引（64位整数）
    pub index: u64,
    /// 单元格 ID（十六进制字符串）
    pub id: String,
    /// 分辨率级别
    pub resolution: u8,
}

impl H3Cell {
    /// 从经纬度创建 H3 单元格
    pub fn from_latlng(lat: f64, lon: f64, resolution: u8) -> Option<Self> {
        let latlng = LatLng::new(lat, lon).ok()?;
        let resolution = Resolution::try_from(resolution).ok()?;
        let cell = latlng.to_cell(resolution);

        Some(Self {
            index: cell.into(),
            id: cell.to_string(),
            resolution: resolution as u8,
        })
    }

    /// 获取该单元格的邻居数量
    pub fn neighbor_count(&self) -> usize {
        6 // 六边形有 6 个邻居
    }
}

/// H3 网格工具
pub struct H3Grid;

impl H3Grid {
    /// 将经纬度转换为 H3 单元格 ID
    pub fn latlng_to_cell(lat: f64, lon: f64, resolution: u8) -> Option<String> {
        let latlng = LatLng::new(lat, lon).ok()?;
        let resolution = Resolution::try_from(resolution).ok()?;
        let cell = latlng.to_cell(resolution);
        Some(cell.to_string())
    }

    /// 将 H3 单元格 ID 转换为经纬度
    pub fn cell_to_latlng(cell_id: &str) -> Option<(f64, f64)> {
        let cell: CellIndex = cell_id.parse().ok()?;
        let latlng = LatLng::from(cell);
        // 使用 lng() 方法获取经度（不是 lon()）
        Some((latlng.lat(), latlng.lng()))
    }

    /// 计算两个 H3 单元格之间的距离（米）
    pub fn cell_distance(cell_id1: &str, cell_id2: &str) -> Option<f64> {
        let cell1: CellIndex = cell_id1.parse().ok()?;
        let cell2: CellIndex = cell_id2.parse().ok()?;
        let latlng1 = LatLng::from(cell1);
        let latlng2 = LatLng::from(cell2);
        // distance_m 需要所有权，使用 clone
        Some(latlng1.distance_m(latlng2))
    }

    /// 检查两个单元格是否相邻
    pub fn are_neighbors(cell_id1: &str, cell_id2: &str) -> bool {
        let cell1: CellIndex = match cell_id1.parse() {
            Ok(c) => c,
            Err(_) => return false,
        };
        let cell2: CellIndex = match cell_id2.parse() {
            Ok(c) => c,
            Err(_) => return false,
        };
        // is_neighbor_with 返回 Result
        cell1.is_neighbor_with(cell2).unwrap_or(false)
    }

    /// 获取区域内所有单元格 ID
    #[allow(dead_code)]
    pub fn cells_in_polygon(
        lat: f64,
        lon: f64,
        resolution: u8,
        radius_km: f64,
    ) -> Vec<String> {
        let latlng = match LatLng::new(lat, lon) {
            Ok(l) => l,
            Err(_) => return vec![],
        };
        
        let resolution = match Resolution::try_from(resolution) {
            Ok(r) => r,
            Err(_) => return vec![],
        };
        
        let cell = latlng.to_cell(resolution);

        // 计算覆盖半径需要的网格距离
        // Level 7 的单元格边长约 4.2km
        let k = (radius_km / 4.0).ceil() as u32;
        
        // 使用 Vec::from_iter 来收集结果
        Vec::from_iter(cell.grid_disk::<Vec<CellIndex>>(k))
            .into_iter()
            .map(|c| c.to_string())
            .collect()
    }
}

/// 将 H3 单元格 ID 转换为用于 GyID 哈希的因子
///
/// 根据 H3 精度级别，生成不同粒度的哈希输入
pub fn to_hash_factor(h3_cell_id: &str, resolution: u8) -> String {
    // H3 单元格 ID 格式如：872830828ffffff
    // 添加分辨率前缀以便区分
    format!("h3:r{}_{}", resolution, h3_cell_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_latlng_to_cell() {
        // 北京天安门坐标
        let lat = 39.9042;
        let lon = 116.4074;
        
        let cell_id = H3Grid::latlng_to_cell(lat, lon, 9);
        assert!(cell_id.is_some());
        println!("北京天安门 H3 Cell (Level 9): {}", cell_id.unwrap());
    }

    #[test]
    fn test_cell_roundtrip() {
        let lat = 39.9042;
        let lon = 116.4074;
        let resolution = 9;
        
        let cell_id = H3Grid::latlng_to_cell(lat, lon, resolution).unwrap();
        let (lat2, lon2) = H3Grid::cell_to_latlng(&cell_id).unwrap();
        
        // 误差应该在可接受范围内（Level 9 的单元格边长约 1km）
        assert!((lat - lat2).abs() < 0.01);
        assert!((lon - lon2).abs() < 0.01);
    }

    #[test]
    fn test_resolution_levels() {
        let lat = 39.9042;
        let lon = 116.4074;
        
        for level in [4, 7, 9, 12, 15] {
            if let Some(cell_id) = H3Grid::latlng_to_cell(lat, lon, level) {
                println!("Level {}: {}", level, cell_id);
            }
        }
    }

    #[test]
    fn test_h3_cell_creation() {
        let cell = H3Cell::from_latlng(39.9042, 116.4074, 9);
        assert!(cell.is_some());
        
        let cell = cell.unwrap();
        println!("H3 Cell: {:?}", cell);
        println!("Neighbor count: {}", cell.neighbor_count());
    }
}
