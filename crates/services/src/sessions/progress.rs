/// Aggregated view of session progress, useful for UI.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionProgress {
    pub total: usize,
    pub answered: usize,
    pub remaining: usize,
    pub is_complete: bool,
}
