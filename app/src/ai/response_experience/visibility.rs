#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EventVisibility {
    UserVisible,
    UserVisibleWhenDetailed,
    DeveloperOnly,
    InternalOnly,
}
