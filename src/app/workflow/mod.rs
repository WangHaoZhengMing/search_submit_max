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
    /// 题目在试卷中的索引（从1开始）
    pub question_index: usize,
    pub screenshot: String,
}

impl QuestionCtx {
    /// 生成日志前缀
    pub fn log_prefix(&self) -> String {
        format!(
            "[试卷#{} 题目#{}]",
            self.paper_index, self.question_index
        )
    }
}
