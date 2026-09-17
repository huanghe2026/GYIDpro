# H3 六边形地理网格协议说明

> 本文档解释 H3 在 GyID 系统中的应用原理

## 1. 什么是 H3？

**H3** 是 Uber 开源的六边形地理网格系统（Hexagonal Hierarchical Spatial Index），将地球表面划分为层次化的六边形网格。

### 核心特性

| 特性 | 描述 |
|------|------|
| **全球覆盖** | 整个地球表面，无缝覆盖 |
| **层次结构** | 16 个精度等级 (0-15) |
| **固定面积** | 同一精度等级下，所有六边形面积相同 |
| **边界对齐** | 网格边界自然对齐道路、建筑 |

## 2. H3 在 GyID 中的应用

### 2.1 位置隐私保护

GyID 使用 H3 单元格 ID 而非原始 GPS 坐标来标识位置：

```
原始坐标: (39.9042, 116.4074)  // 北京天安门
     ↓
H3 单元格: 8a2a100c3fffff       // 分辨率 7，对应约 5.16 km²
```

### 2.2 精度等级映射

GyID 的三级精度系统与 H3 分辨率对应关系：

| GyID 精度 | H3 分辨率 | 单元格面积 | 适用场景 |
|-----------|-----------|------------|----------|
| L1 城市级 | 3-4 | 252-1107 km² | 粗略位置 |
| L2 区域级 | 6-7 | 4-10 km² | 社区级 |
| L3 精确级 | 9-10 | 0.1-0.4 km² | 街道级 |

### 2.3 单元格计算

```rust
// GyID 中 H3 单元格计算示例
pub fn calculate_h3_cell(&mut self) {
    if let (Some(lat), Some(lon)) = (self.latitude, self.longitude) {
        // 精度 7: 约 5.16 km² 的六边形
        if let Some(cell) = h3o::LatLng::new(lat, lon)
            .ok()
            .and_then(|ll| ll.to_cell(h3o::resolution(7)).ok())
        {
            self.h3_cell = Some(cell.to_string());
        }
    }
}
```

## 3. H3 编码格式

### 3.1 H3 索引结构

H3 索引是一个 64 位整数，编码了以下信息：

```
┌──────────────────────────────────────────────┌──────────┐
│                 45 bits                       │  4 bits  │  15 bits
├──────────────────────────────────────────────┼──────────┤
│           Cell Index ( направление )          │ Resolution│   Reserved
└──────────────────────────────────────────────┴──────────┘
```

### 3.2 字符串表示

H3 单元格可以用不同格式表示：

| 格式 | 示例 | 长度 |
|------|------|------|
| **Hex** | `8a2a100c3fffff` | 15 字符 |
| **Int64** | `609283209348276223` | 数字 |

GyID 使用 **Hex 格式**存储 H3 单元格 ID。

## 4. 层次化结构

### 4.1 分辨率等级

H3 提供 16 个精度等级（0-15），每个等级将上一级六边形细分为 7 个：

```
分辨率 0:  整个地球 = 122 个六边形（极区无效）
分辨率 1:  每个0级 = 7 个
分辨率 2:  每个1级 = 7 个
...
分辨率 15: 每个14级 = 7 个 (约 0.9 m²)
```

### 4.2 父级/子级关系

```
        ┌─────────────────┐
        │   Res 5 (父)     │
        │   ~154 km²       │
        └────────┬────────┘
                 │
    ┌────────────┼────────────┐
    │            │            │
    ▼            ▼            ▼
┌───────┐  ┌───────┐  ┌───────┐
│Res 6  │  │Res 6  │  │Res 6  │
│ ~22km²│  │ ~22km²│  │ ~22km²│
└───────┘  └───────┘  └───────┘
```

可以通过 `cell.parent(resolution)` 获取任意精度的父单元格。

## 5. GyID 中的实现

### 5.1 数据存储

```rust
pub struct GeoLocation {
    pub latitude: Option<f64>,
    pub longitude: Option<f64>,
    pub h3_cell: Option<String>,      // H3 单元格 ID
    pub h3_resolution: u8,            // H3 分辨率
    pub city: Option<String>,
    pub country: Option<String>,
}
```

### 5.2 位置哈希因子

GyID 使用 H3 单元格 ID 生成位置哈希因子：

```rust
pub fn to_hash_factor(&self) -> String {
    let mut factor = String::new();
    
    // 添加 H3 单元格（如果存在）
    if let Some(cell) = &self.h3_cell {
        factor.push_str(cell);
    }
    
    // 添加精度等级
    factor.push_str(&format!("_r{}", self.h3_resolution));
    
    factor
}
```

## 6. 隐私设计

### 6.1 为什么使用 H3？

| 方案 | 优点 | 缺点 |
|------|------|------|
| **原始 GPS** | 精确 | 隐私风险高 |
| **H3 单元格** | 精度可控、面积固定 | 轻微信息损失 |
| **行政区划** | 语义清晰 | 边界不规则 |

### 6.2 H3 的隐私优势

1. **固定精度**: 同一单元格内的所有点产生相同的 H3 ID
2. **不可逆**: 无法从 H3 ID 还原精确坐标
3. **层次选择**: 用户可选择暴露的精度级别

## 7. 与其他位置编码对比

| 编码 | 网格形状 | 层次级别 | 存储大小 | 支持分析 |
|------|----------|----------|----------|----------|
| **H3** | 六边形 | 16 级 | 8 bytes | ✅ 优秀 |
| **Geohash** | 矩形 | 12 级 | 6-11 chars | ⚠️ 一般 |
| **S2** | 正方形 | 30 级 | 8 bytes | ✅ 优秀 |
| **Tile38** | 多边形 | 无限 | 可变 | ✅ 优秀 |

## 8. 使用示例

### 8.1 获取当前位置的 H3 单元格

```rust
use h3o::LatLng;

let location = LatLng::new(39.9042, 116.4074)?;
let cell = location.to_cell(h3o::resolution(7))?;
println!("H3 Cell: {}", cell.to_string());
// 输出: 8a2a100c3fffff
```

### 8.2 计算两个位置的关系

```rust
use h3o::{Cell, LatLng};

// 北京和杭州
let beijing = LatLng::new(39.9042, 116.4074)?.to_cell(h3o::resolution(7));
let hangzhou = LatLng::new(30.2741, 120.1551)?.to_cell(h3o::resolution(7));

// 检查是否在同一个单元格
if beijing == hangzhou {
    println!("同一区域");
}

// 计算距离（km）
let distance = beijing.boundary().center().distance_m(hangzhou.boundary().center())? / 1000.0;
```

### 8.3 网格可视化

```rust
// 获取单元格的六个顶点坐标
let boundary = cell.boundary();
for point in boundary {
    println!("({}, {})", point.lat(), point.lng());
}
```

## 9. 性能注意事项

| 操作 | 时间复杂度 | 说明 |
|------|------------|------|
| 坐标→单元格 | O(1) | 常数时间 |
| 单元格→坐标 | O(1) | 取中心点 |
| 网格邻居 | O(1) | 最多 7 个 |
| 父/子单元格 | O(1) | 直接计算 |

## 10. 相关资源

- **H3 官网**: https://h3geo.org/
- **H3 Rust 库**: https://github.com/nicolaseberle/h3o (gyid-core 使用)
- **H3 可视化**: https://uber.github.io/h3/

---

*文档版本: v1.0*  
*更新时间: 2026-04-10*
