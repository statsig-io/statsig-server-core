use std::fmt::Display;

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum FlushType {
    ScheduledMaxTime,
    ScheduledFullBatch,
    Limit,
    Manual,
    Shutdown,
}

impl Display for FlushType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FlushType::ScheduledMaxTime => "scheduled:max_time",
            FlushType::ScheduledFullBatch => "scheduled:full_batch",
            FlushType::Limit => "limit",
            FlushType::Manual => "manual",
            FlushType::Shutdown => "shutdown",
        }
        .fmt(f)
    }
}
