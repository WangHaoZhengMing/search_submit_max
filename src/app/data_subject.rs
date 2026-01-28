

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Subject {
    Chinese,   // 语文
    Math,      // 数学
    English,   // 英语
    Physics,   // 物理
    Chemistry, // 化学
    Biology,   // 生物
    History,   // 历史
    Politics,  // 政治
    Geography, // 地理
    Science,   // 科学
}

impl Subject {
    /// 获取科目代码
    pub fn code(&self) -> i16 {
        match self {
            Subject::Chinese => 55,
            Subject::Math => 54,
            Subject::English => 53,
            Subject::Physics => 56,
            Subject::Chemistry => 57,
            Subject::Biology => 58,
            Subject::History => 61,
            Subject::Politics => 60,
            Subject::Geography => 59,
            Subject::Science => 62,
        }
    }

    /// 从完整名称获取科目
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "语文" => Some(Subject::Chinese),
            "数学" => Some(Subject::Math),
            "英语" => Some(Subject::English),
            "物理" => Some(Subject::Physics),
            "化学" => Some(Subject::Chemistry),
            "生物" => Some(Subject::Biology),
            "历史" => Some(Subject::History),
            "政治" => Some(Subject::Politics),
            "地理" => Some(Subject::Geography),
            "科学" => Some(Subject::Science),
            _ => None,
        }
    }

    /// 从简写获取科目
    pub fn from_short_name(name: &str) -> Option<Self> {
        match name {
            "语" => Some(Subject::Chinese),
            "数" => Some(Subject::Math),
            "英" => Some(Subject::English),
            "物" => Some(Subject::Physics),
            "化" => Some(Subject::Chemistry),
            "生" => Some(Subject::Biology),
            "历" => Some(Subject::History),
            "政" => Some(Subject::Politics),
            "地" => Some(Subject::Geography),
            "科" => Some(Subject::Science),
            _ => None,
        }
    }
}

/// 获取科目code
pub fn get_subject_code(subject_name: &str) -> Option<i16> {
    Subject::from_name(subject_name).map(|s| s.code())
}

/// 智能查找科目code（支持简写）
pub fn smart_find_subject_code(name: &str) -> Option<i16> {
    // 先尝试完整名称
    if let Some(code) = get_subject_code(name) {
        return Some(code);
    }
    // 尝试简写匹配
    Subject::from_short_name(name).map(|s| s.code())
}
