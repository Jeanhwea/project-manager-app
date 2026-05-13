#[derive(Debug, Clone)]
pub enum MessageOperation {
    Header {
        title: String,
    },
    Section {
        title: String,
    },
    Item {
        label: String,
        value: String,
    },
    Detail {
        label: String,
        value: String,
    },
    Diff {
        file: String,
        line_num: usize,
        old_content: String,
        new_content: String,
    },
    Success {
        msg: String,
    },
    Warning {
        msg: String,
    },
    Skip {
        msg: String,
    },
    Blank,
}

impl MessageOperation {
    pub fn description(&self) -> String {
        match self {
            MessageOperation::Header { title } => title.clone(),
            MessageOperation::Section { title } => title.clone(),
            MessageOperation::Item { label, value } => format!("{}: {}", label, value),
            MessageOperation::Detail { label, value } => format!("  {}: {}", label, value),
            MessageOperation::Diff {
                file,
                line_num,
                old_content,
                new_content,
            } => format!(
                "{} L{} -:  {}\n{} L{} +:  {}",
                file, line_num, old_content, file, line_num, new_content
            ),
            MessageOperation::Success { msg } => format!("OK> {}", msg),
            MessageOperation::Warning { msg } => format!("WARN {}", msg),
            MessageOperation::Skip { msg } => format!("SKIP {}", msg),
            MessageOperation::Blank => String::new(),
        }
    }
}
