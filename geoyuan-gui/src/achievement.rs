// ═══════════════════════════════════════════════════════════════════════════
// GeoYuan - 成就系统
// 游戏化激励 · 徽章解锁
// ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use chrono::Timelike;

/// 成就类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AchievementType {
    FirstMint,         // 首次铸造
    SevenDayStreak,    // 连续7天
    Explorer,          // 探索者 - 跨城市
    Collector,         // 收藏家 - 10个GyID
    Veteran,           // 老兵 - 100个GyID
    EarlyBird,         // 早起鸟 - 凌晨5-7点铸造
    NightOwl,          // 夜猫子 - 凌晨0-5点铸造
    Traveler,          // 旅行者 - 10个不同城市
    RichMan,           // 富翁 - 持有100 GY
}

/// 成就信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    pub name: String,
    pub description: String,
    pub icon: String,       // emoji 图标
    pub progress: f32,     // 0.0 - 1.0
    pub unlocked: bool,
    pub unlocked_at: Option<i64>,  // 时间戳
}

impl Achievement {
    pub fn new(id: AchievementType, name: &str, description: &str, icon: &str) -> Self {
        Self {
            id: format!("{:?}", id),
            name: name.to_string(),
            description: description.to_string(),
            icon: icon.to_string(),
            progress: 0.0,
            unlocked: false,
            unlocked_at: None,
        }
    }
}

/// 成就管理器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementManager {
    pub achievements: HashMap<String, Achievement>,
    pub total_gyid_count: u32,
    pub cities_visited: Vec<String>,
    pub current_streak: u32,
    pub last_mint_date: Option<i64>,
}

impl Default for AchievementManager {
    fn default() -> Self {
        let mut achievements = HashMap::new();
        
        // 初始化所有成就
        achievements.insert(
            "FirstMint".to_string(),
            Achievement::new(AchievementType::FirstMint, "初次见面", "完成首次 GyID 铸造", "🎉")
        );
        achievements.insert(
            "SevenDayStreak".to_string(),
            Achievement::new(AchievementType::SevenDayStreak, "坚持不懈", "连续7天铸造 GyID", "🔥")
        );
        achievements.insert(
            "Explorer".to_string(),
            Achievement::new(AchievementType::Explorer, "探索者", "在2个以上不同城市铸造", "🌍")
        );
        achievements.insert(
            "Collector".to_string(),
            Achievement::new(AchievementType::Collector, "收藏家", "铸造10个 GyID", "🗃️")
        );
        achievements.insert(
            "Veteran".to_string(),
            Achievement::new(AchievementType::Veteran, "老兵", "铸造100个 GyID", "🎖️")
        );
        achievements.insert(
            "EarlyBird".to_string(),
            Achievement::new(AchievementType::EarlyBird, "早起鸟", "在凌晨5-7点铸造", "🌅")
        );
        achievements.insert(
            "NightOwl".to_string(),
            Achievement::new(AchievementType::NightOwl, "夜猫子", "在凌晨0-5点铸造", "🦉")
        );
        achievements.insert(
            "Traveler".to_string(),
            Achievement::new(AchievementType::Traveler, "旅行者", "在10个不同城市铸造", "✈️")
        );
        achievements.insert(
            "RichMan".to_string(),
            Achievement::new(AchievementType::RichMan, "富翁", "持有100 GY", "💰")
        );

        Self {
            achievements,
            total_gyid_count: 0,
            cities_visited: Vec::new(),
            current_streak: 0,
            last_mint_date: None,
        }
    }
}

impl AchievementManager {
    /// 检查并更新成就进度
    pub fn check_achievements(&mut self, city: &str, gy_count: f32, timestamp: i64) -> Vec<String> {
        let mut unlocked = Vec::new();
        let now = chrono::Utc::now();
        let hour = now.hour();

        // 更新 GyID 计数
        self.total_gyid_count += 1;

        // 记录城市
        if !self.cities_visited.contains(&city.to_string()) {
            self.cities_visited.push(city.to_string());
        }

        // 1. 首次铸造
        if self.total_gyid_count == 1 {
            if let Some(ach) = self.achievements.get_mut("FirstMint") {
                ach.progress = 1.0;
                ach.unlocked = true;
                ach.unlocked_at = Some(timestamp);
                unlocked.push("FirstMint".to_string());
            }
        }

        // 2. 收藏家 (10个)
        if let Some(ach) = self.achievements.get_mut("Collector") {
            ach.progress = (self.total_gyid_count as f32 / 10.0).min(1.0);
            if self.total_gyid_count >= 10 && !ach.unlocked {
                ach.unlocked = true;
                ach.unlocked_at = Some(timestamp);
                unlocked.push("Collector".to_string());
            }
        }

        // 3. 老兵 (100个)
        if let Some(ach) = self.achievements.get_mut("Veteran") {
            ach.progress = (self.total_gyid_count as f32 / 100.0).min(1.0);
            if self.total_gyid_count >= 100 && !ach.unlocked {
                ach.unlocked = true;
                ach.unlocked_at = Some(timestamp);
                unlocked.push("Veteran".to_string());
            }
        }

        // 4. 探索者 (2个城市)
        if let Some(ach) = self.achievements.get_mut("Explorer") {
            ach.progress = (self.cities_visited.len() as f32 / 2.0).min(1.0);
            if self.cities_visited.len() >= 2 && !ach.unlocked {
                ach.unlocked = true;
                ach.unlocked_at = Some(timestamp);
                unlocked.push("Explorer".to_string());
            }
        }

        // 5. 旅行者 (10个城市)
        if let Some(ach) = self.achievements.get_mut("Traveler") {
            ach.progress = (self.cities_visited.len() as f32 / 10.0).min(1.0);
            if self.cities_visited.len() >= 10 && !ach.unlocked {
                ach.unlocked = true;
                ach.unlocked_at = Some(timestamp);
                unlocked.push("Traveler".to_string());
            }
        }

        // 6. 早起鸟 (5-7点)
        if let Some(ach) = self.achievements.get_mut("EarlyBird") {
            if (5..=7).contains(&hour) {
                ach.progress = 1.0;
                if !ach.unlocked {
                    ach.unlocked = true;
                    ach.unlocked_at = Some(timestamp);
                    unlocked.push("EarlyBird".to_string());
                }
            }
        }

        // 7. 夜猫子 (0-5点)
        if let Some(ach) = self.achievements.get_mut("NightOwl") {
            if (0..=5).contains(&hour) {
                ach.progress = 1.0;
                if !ach.unlocked {
                    ach.unlocked = true;
                    ach.unlocked_at = Some(timestamp);
                    unlocked.push("NightOwl".to_string());
                }
            }
        }

        // 8. 富翁 (100 GY)
        if let Some(ach) = self.achievements.get_mut("RichMan") {
            ach.progress = (gy_count / 100.0).min(1.0);
            if gy_count >= 100.0 && !ach.unlocked {
                ach.unlocked = true;
                ach.unlocked_at = Some(timestamp);
                unlocked.push("RichMan".to_string());
            }
        }

        unlocked
    }

    /// 获取已解锁成就数量
    pub fn unlocked_count(&self) -> usize {
        self.achievements.values().filter(|a| a.unlocked).count()
    }

    /// 获取总成就数量
    pub fn total_count(&self) -> usize {
        self.achievements.len()
    }

    /// 获取最近解锁的成就
    #[allow(dead_code)]
    pub fn recent_unlocked(&self, limit: usize) -> Vec<&Achievement> {
        let mut unlocked: Vec<_> = self.achievements.values()
            .filter(|a| a.unlocked)
            .collect();
        
        unlocked.sort_by(|a, b| {
            b.unlocked_at.cmp(&a.unlocked_at)
        });
        
        unlocked.into_iter().take(limit).collect()
    }

    /// 获取所有成就列表
    #[allow(dead_code)]
    pub fn get_all(&self) -> Vec<&Achievement> {
        self.achievements.values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_first_mint() {
        let mut manager = AchievementManager::default();
        let unlocked = manager.check_achievements("深圳", 1.0, 0);
        assert!(unlocked.contains(&"FirstMint".to_string()));
    }

    #[test]
    fn test_progress() {
        let mut manager = AchievementManager::default();
        // 通过 check_achievements 驱动状态变化，模拟铸造 5 个 GyID
        for _ in 0..5 {
            manager.check_achievements("深圳", 1.0, 0);
        }
        
        if let Some(ach) = manager.achievements.get("Collector") {
            // total_gyid_count == 5，Collector 目标 10，进度应为 0.5
            assert_eq!(ach.progress, 0.5);
        }
    }
}
