#[derive(Debug, Clone, Default)]
pub struct RuleValidationReport {
    pub status: RuleValidationStatus,
    pub estimated_tokens: u32,
    pub warnings: Vec<RuleValidationWarning>,
    pub errors: Vec<RuleValidationError>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum RuleValidationStatus {
    #[default]
    Valid,
    ValidWithWarnings,
    Invalid,
}

#[derive(Debug, Clone)]
pub enum RuleValidationWarning {
    OversizedRule,
    DuplicateRuleSource,
    ConflictingSource,
    UnsupportedMetadata,
}

#[derive(Debug, Clone)]
pub enum RuleValidationError {
    BrokenInclude,
    InvalidPathScope,
    UnreadableFile,
    InvalidEncoding,
}
