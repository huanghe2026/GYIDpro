//! H3 六边形网格命令

use anyhow::Result;
use clap::Parser;
use h3o::{CellIndex, LatLng, Resolution};

/// H3 六边形网格工具
#[derive(Parser, Debug)]
#[command(name = "h3")]
#[command(about = "H3 六边形地理网格工具", long_about = None)]
pub struct H3Command {
    /// 子命令
    #[command(subcommand)]
    pub subcommand: H3SubCommand,
}

#[derive(Parser, Debug, Clone)]
pub enum H3SubCommand {
    /// 将经纬度转换为 H3 单元格
    #[command(name = "encode")]
    Encode {
        /// 纬度
        #[arg(short = 'a', long)]
        lat: f64,
        /// 经度
        #[arg(short = 'o', long)]
        lon: f64,
        /// H3 分辨率级别 (0-15，默认 9)
        #[arg(short, long, default_value_t = 9)]
        level: u8,
    },
    /// 将 H3 单元格 ID 转换为经纬度
    #[command(name = "decode")]
    Decode {
        /// H3 单元格 ID
        #[arg(short, long)]
        cell: String,
    },
    /// 获取单元格的邻居
    #[command(name = "neighbors")]
    Neighbors {
        /// H3 单元格 ID
        #[arg(short, long)]
        cell: String,
    },
    /// 获取单元格的父单元格（上一级）
    #[command(name = "parent")]
    Parent {
        /// H3 单元格 ID
        #[arg(short, long)]
        cell: String,
    },
    /// 获取单元格的子单元格（下一级）
    #[command(name = "children")]
    Children {
        /// H3 单元格 ID
        #[arg(short, long)]
        cell: String,
    },
    /// 测试不同精度级别
    #[command(name = "levels")]
    Levels {
        /// 纬度
        #[arg(short = 'a', long)]
        lat: f64,
        /// 经度
        #[arg(short = 'o', long)]
        lon: f64,
    },
    /// 计算两个单元格之间的距离
    #[command(name = "distance")]
    Distance {
        /// 第一个 H3 单元格 ID
        #[arg(short = '1', long)]
        cell1: String,
        /// 第二个 H3 单元格 ID
        #[arg(short = '2', long)]
        cell2: String,
    },
    /// 检查两个单元格是否相邻
    #[command(name = "adjacent")]
    Adjacent {
        /// 第一个 H3 单元格 ID
        #[arg(short = '1', long)]
        cell1: String,
        /// 第二个 H3 单元格 ID
        #[arg(short = '2', long)]
        cell2: String,
    },
}

impl H3Command {
    pub async fn run(&self) -> Result<()> {
        match &self.subcommand {
            H3SubCommand::Encode { lat, lon, level } => {
                self.cmd_encode(*lat, *lon, *level)?;
            }
            H3SubCommand::Decode { cell } => {
                self.cmd_decode(cell)?;
            }
            H3SubCommand::Neighbors { cell } => {
                self.cmd_neighbors(cell)?;
            }
            H3SubCommand::Parent { cell } => {
                self.cmd_parent(cell)?;
            }
            H3SubCommand::Children { cell } => {
                self.cmd_children(cell)?;
            }
            H3SubCommand::Levels { lat, lon } => {
                self.cmd_levels(*lat, *lon)?;
            }
            H3SubCommand::Distance { cell1, cell2 } => {
                self.cmd_distance(cell1, cell2)?;
            }
            H3SubCommand::Adjacent { cell1, cell2 } => {
                self.cmd_adjacent(cell1, cell2)?;
            }
        }
        Ok(())
    }

    fn cmd_encode(&self, lat: f64, lon: f64, level: u8) -> Result<()> {
        println!("\n=== H3 编码转换 ===");
        println!("输入坐标: lat={}, lon={}", lat, lon);
        println!("分辨率级别: {}", level);

        match latlng_to_cell(lat, lon, level) {
            Some(cell_id) => {
                println!("\nH3 单元格 ID: {}", cell_id);
                
                // 获取单元格详细信息
                if let Some(cell) = H3Cell::from_latlng(lat, lon, level) {
                    println!("单元格索引: {}", cell.index);
                    println!("中心点: ({:.6}, {:.6})", cell.lat, cell.lon);
                }
                
                // 显示分辨率描述
                println!("精度描述: Level {} (约 {:.2} m2)", level, resolution_area_m2(level));
            }
            None => {
                println!("转换失败，请检查坐标范围");
            }
        }
        
        println!("====================\n");
        Ok(())
    }

    fn cmd_decode(&self, cell_id: &str) -> Result<()> {
        println!("\n=== H3 解码转换 ===");
        println!("H3 单元格 ID: {}", cell_id);

        match cell_to_latlng(cell_id) {
            Some((lat, lon)) => {
                println!("\n解码结果:");
                println!("  纬度:  {:.6}", lat);
                println!("  经度:  {:.6}", lon);
            }
            None => {
                println!("解码失败，请检查单元格 ID 格式");
            }
        }
        
        println!("====================\n");
        Ok(())
    }

    fn cmd_neighbors(&self, cell_id: &str) -> Result<()> {
        println!("\n=== H3 邻居查询 ===");
        println!("H3 单元格 ID: {}", cell_id);

        let cell_index: Result<CellIndex, _> = cell_id.parse();
        match cell_index {
            Ok(idx) => {
                let neighbors: Vec<String> = idx.grid_disk::<Vec<_>>(1)
                    .into_iter()
                    .filter(|c| *c != idx)
                    .map(|c| c.to_string())
                    .collect();
                
                println!("\n邻居单元格 (共 {} 个):", neighbors.len());
                for (i, neighbor) in neighbors.iter().enumerate() {
                    println!("  {}. {}", i + 1, neighbor);
                }
            }
            Err(_) => {
                println!("无效的单元格 ID 格式");
            }
        }
        
        println!("====================\n");
        Ok(())
    }

    fn cmd_parent(&self, cell_id: &str) -> Result<()> {
        println!("\n=== H3 父单元格查询 ===");
        println!("H3 单元格 ID: {}", cell_id);

        let cell_index: Result<CellIndex, _> = cell_id.parse();
        match cell_index {
            Ok(idx) => {
                let res = idx.resolution();
                let res_u8: u8 = res.into();
                
                if res_u8 == 0 {
                    println!("已经是最高级别（Level 0），没有父单元格");
                } else {
                    let parent_res = Resolution::try_from(res_u8 - 1)
                        .expect("Invalid resolution");
                    
                    match idx.parent(parent_res) {
                        Some(parent) => {
                            println!("\n父单元格:");
                            println!("  Level {}: {}", res_u8 - 1, parent);
                        }
                        None => {
                            println!("无法获取父单元格");
                        }
                    }
                }
            }
            Err(_) => {
                println!("无效的单元格 ID 格式");
            }
        }
        
        println!("====================\n");
        Ok(())
    }

    fn cmd_children(&self, cell_id: &str) -> Result<()> {
        println!("\n=== H3 子单元格查询 ===");
        println!("H3 单元格 ID: {}", cell_id);

        let cell_index: Result<CellIndex, _> = cell_id.parse();
        match cell_index {
            Ok(idx) => {
                let res = idx.resolution();
                let res_u8: u8 = res.into();
                
                if res_u8 >= 15 {
                    println!("已经是最低级别（Level 15），没有子单元格");
                } else {
                    let child_res = Resolution::try_from(res_u8 + 1)
                        .expect("Invalid resolution");
                    
                    let children: Vec<String> = idx.children(child_res)
                        .map(|c| c.to_string())
                        .collect();
                    
                    println!("\n子单元格 (Level {}, 共 {} 个):", res_u8 + 1, children.len());
                    for (i, child) in children.iter().take(7).enumerate() {
                        println!("  {}. {}", i + 1, child);
                    }
                    if children.len() > 7 {
                        println!("  ... 还有 {} 个", children.len() - 7);
                    }
                }
            }
            Err(_) => {
                println!("无效的单元格 ID 格式");
            }
        }
        
        println!("====================\n");
        Ok(())
    }

    fn cmd_levels(&self, lat: f64, lon: f64) -> Result<()> {
        println!("\n=== H3 各精度级别示例 ===");
        println!("坐标: lat={}, lon={}", lat, lon);
        println!("\n各分辨率级别的 H3 单元格:\n");

        for level in [0u8, 4, 7, 9, 12, 15] {
            if let Some(cell_id) = latlng_to_cell(lat, lon, level) {
                println!("Level {:2}: {}  (~{:.2} m2)", level, cell_id, resolution_area_m2(level));
            }
        }

        println!("\nGyID 精度等级映射:");
        println!("  City    -> Level 4  (~252 km2)");
        println!("  District -> Level 7  (~38 km2)");
        println!("  Exact   -> Level 9  (~0.74 km2)");
        
        println!("====================\n");
        Ok(())
    }

    fn cmd_distance(&self, cell1: &str, cell2: &str) -> Result<()> {
        println!("\n=== H3 距离计算 ===");
        println!("单元格 1: {}", cell1);
        println!("单元格 2: {}", cell2);

        match cell_distance(cell1, cell2) {
            Some(distance) => {
                println!("\n两单元格中心点距离:");
                println!("  {:.2} 米", distance);
                if distance >= 1000.0 {
                    println!("  ({:.2} 公里)", distance / 1000.0);
                }
            }
            None => {
                println!("距离计算失败，请检查单元格 ID 格式");
            }
        }
        
        println!("====================\n");
        Ok(())
    }

    fn cmd_adjacent(&self, cell1: &str, cell2: &str) -> Result<()> {
        println!("\n=== H3 邻接检查 ===");
        println!("单元格 1: {}", cell1);
        println!("单元格 2: {}", cell2);

        if are_neighbors(cell1, cell2) {
            println!("\n两个单元格相邻");
        } else {
            println!("\n两个单元格不相邻");
        }
        
        println!("====================\n");
        Ok(())
    }
}

// ============ H3 辅助函数 ============

/// H3 单元格信息
#[derive(Debug, Clone)]
pub struct H3Cell {
    pub index: u64,
    pub lat: f64,
    pub lon: f64,
    #[allow(dead_code)]
    pub resolution: u8, // 保留供将来使用
}

impl H3Cell {
    /// 从经纬度创建 H3 单元格
    pub fn from_latlng(lat: f64, lon: f64, resolution: u8) -> Option<Self> {
        let latlng = LatLng::new(lat, lon).ok()?;
        let resolution = Resolution::try_from(resolution).ok()?;
        let cell = latlng.to_cell(resolution);

        let latlng_out = LatLng::from(cell);
        Some(Self {
            index: cell.into(),
            lat: latlng_out.lat(),
            lon: latlng_out.lng(),
            resolution: resolution as u8,
        })
    }
}

/// 将经纬度转换为 H3 单元格 ID
fn latlng_to_cell(lat: f64, lon: f64, resolution: u8) -> Option<String> {
    let latlng = LatLng::new(lat, lon).ok()?;
    let resolution = Resolution::try_from(resolution).ok()?;
    let cell = latlng.to_cell(resolution);
    Some(cell.to_string())
}

/// 将 H3 单元格 ID 转换为经纬度
fn cell_to_latlng(cell_id: &str) -> Option<(f64, f64)> {
    let cell: CellIndex = cell_id.parse().ok()?;
    let latlng = LatLng::from(cell);
    Some((latlng.lat(), latlng.lng()))
}

/// 计算两个 H3 单元格之间的距离（米）
fn cell_distance(cell_id1: &str, cell_id2: &str) -> Option<f64> {
    let cell1: CellIndex = cell_id1.parse().ok()?;
    let cell2: CellIndex = cell_id2.parse().ok()?;
    let latlng1 = LatLng::from(cell1);
    let latlng2 = LatLng::from(cell2);
    Some(latlng1.distance_m(latlng2))
}

/// 检查两个单元格是否相邻
fn are_neighbors(cell_id1: &str, cell_id2: &str) -> bool {
    let cell1: CellIndex = match cell_id1.parse() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let cell2: CellIndex = match cell_id2.parse() {
        Ok(c) => c,
        Err(_) => return false,
    };
    cell1.is_neighbor_with(cell2).unwrap_or(false)
}

/// 获取 H3 分辨率的平均面积（平方米）
fn resolution_area_m2(level: u8) -> f64 {
    match level {
        0 => 4.25e12,
        1 => 6.08e11,
        2 => 8.72e10,
        3 => 1.25e10,
        4 => 1.79e9,
        5 => 2.56e8,
        6 => 3.66e7,
        7 => 5.23e6,
        8 => 7.47e5,
        9 => 1.07e5,
        10 => 15274.0,
        11 => 2182.0,
        12 => 311.7,
        13 => 44.5,
        14 => 6.36,
        15 => 0.91,
        _ => 0.0,
    }
}
