pub mod pipeline;
pub mod process_single;

pub struct QuestionCtx {
    pub paper_id: String,
    /// 科目代码
    pub subject_code: String,
    /// 学段代码（如 "3" 表示高中）
    pub stage: String,
    /// 试卷索引（仅用于日志显示）
    pub paper_index: usize,
    /// 题目在试卷中的索引（从1开始，title和普通题目统一编号）
    pub question_index: usize,
    /// 是否为标题题目
    pub is_title: bool,
    pub screenshot: String,
}

impl QuestionCtx {
    /// 生成日志前缀
    pub fn log_prefix(&self) -> String {
        let type_str = if self.is_title { "标题" } else { "题目" };
        format!(
            "[试卷#{} {}#{}]",
            self.paper_index, type_str, self.question_index
        )
    }
}
