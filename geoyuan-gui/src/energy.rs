// ═══════════════════════════════════════════════════════════════════════════
// GeoYuan - 能量系统
// 游戏化激励 · 每日能量
// ═══════════════════════════════════════════════════════════════════════════

use serde::{Deserialize, Serialize};

/// 每日任务类型
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum TaskType {
    DailyMint,      // 每日铸造
    Share,          // 分享
    Verify,         // 验证他人
    FirstLogin,     // 首次登录
}

/// 任务状态
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    pub task_type: TaskType,
    pub name: String,
    pub description: String,
    pub reward: f32,        // 奖励能量
    pub completed: bool,
    pub progress: f32,      // 0.0 - 1.0
}

/// 能量数据
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnergyData {
    pub current: f32,           // 当前能量
    pub max: f32,               // 最大能量
    pub last_daily_reset: i64,   // 上次每日重置时间戳
    pub total_earned: f32,       // 总获得能量
    pub total_spent: f32,       // 总消耗能量
}

impl Default for EnergyData {
    fn default() -> Self {
        Self {
            current: 50.0,       // 初始50能量
            max: 100.0,          // 上限100
            last_daily_reset: 0,
            total_earned: 50.0,
            total_spent: 0.0,
        }
    }
}

impl EnergyData {
    /// 消耗能量
    #[allow(dead_code)]
    pub fn consume(&mut self, amount: f32) -> bool {
        if self.current >= amount {
            self.current -= amount;
            self.total_spent += amount;
            true
        } else {
            false
        }
    }

    /// 增加能量
    pub fn add(&mut self, amount: f32) {
        self.current = (self.current + amount).min(self.max);
        self.total_earned += amount;
    }

    /// 每日重置检查
    pub fn check_daily_reset(&mut self, current_timestamp: i64) -> bool {
        let one_day_seconds = 86400;
        
        if current_timestamp - self.last_daily_reset >= one_day_seconds {
            // 每日重置：恢复20能量，但不超过上限
            self.add(20.0);
            self.last_daily_reset = current_timestamp;
            true
        } else {
            false
        }
    }

    /// 获取能量百分比
    #[allow(dead_code)]
    pub fn percentage(&self) -> f32 {
        self.current / self.max
    }
}

/// 每日任务管理器
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskManager {
    pub tasks: Vec<Task>,
    pub last_login_date: Option<i64>,
}

impl Default for TaskManager {
    fn default() -> Self {
        let tasks = vec![
            Task {
                id: "daily_mint".to_string(),
                task_type: TaskType::DailyMint,
                name: "每日铸造".to_string(),
                description: "今日铸造至少1个 GyID".to_string(),
                reward: 10.0,
                completed: false,
                progress: 0.0,
            },
            Task {
                id: "share".to_string(),
                task_type: TaskType::Share,
                name: "分享传播".to_string(),
                description: "分享 GyID 给好友".to_string(),
                reward: 5.0,
                completed: false,
                progress: 0.0,
            },
            Task {
                id: "verify".to_string(),
                task_type: TaskType::Verify,
                name: "验证达人".to_string(),
                description: "验证3个其他 GyID".to_string(),
                reward: 8.0,
                completed: false,
                progress: 0.0,
            },
            Task {
                id: "first_login".to_string(),
                task_type: TaskType::FirstLogin,
                name: "每日登录".to_string(),
                description: "每日首次打开应用".to_string(),
                reward: 2.0,
                completed: false,
                progress: 0.0,
            },
        ];

        Self {
            tasks,
            last_login_date: None,
        }
    }
}

impl TaskManager {
    /// 检查任务完成
    pub fn complete_task(&mut self, task_id: &str) -> Option<f32> {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            if !task.completed {
                task.completed = true;
                task.progress = 1.0;
                return Some(task.reward);
            }
        }
        None
    }

    /// 更新任务进度
    #[allow(dead_code)]
    pub fn update_progress(&mut self, task_id: &str, progress: f32) {
        if let Some(task) = self.tasks.iter_mut().find(|t| t.id == task_id) {
            task.progress = progress.min(1.0);
            if task.progress >= 1.0 && !task.completed {
                task.completed = true;
            }
        }
    }

    /// 每日重置任务
    pub fn reset_daily(&mut self) {
        for task in &mut self.tasks {
            // 保留首日登录任务的完成状态
            if task.task_type != TaskType::FirstLogin {
                task.completed = false;
                task.progress = 0.0;
            }
        }
    }

    /// 获取已完成任务数
    #[allow(dead_code)]
    pub fn completed_count(&self) -> usize {
        self.tasks.iter().filter(|t| t.completed).count()
    }

    /// 获取任务总数
    #[allow(dead_code)]
    pub fn total_count(&self) -> usize {
        self.tasks.len()
    }

    /// 获取可获得的奖励
    #[allow(dead_code)]
    pub fn pending_rewards(&self) -> f32 {
        self.tasks.iter()
            .filter(|t| !t.completed)
            .map(|t| t.reward)
            .sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_energy_consume() {
        let mut energy = EnergyData::default();
        assert!(energy.consume(30.0));
        assert_eq!(energy.current, 20.0);
    }

    #[test]
    fn test_energy_not_enough() {
        let mut energy = EnergyData::default();
        assert!(!energy.consume(100.0));
    }

    #[test]
    fn test_task_complete() {
        let mut manager = TaskManager::default();
        let reward = manager.complete_task("daily_mint");
        assert_eq!(reward, Some(10.0));
    }
}
