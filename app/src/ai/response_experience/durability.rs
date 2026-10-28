#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EventDurability {
    Transient,
    Checkpointed,
    Durable,
}
