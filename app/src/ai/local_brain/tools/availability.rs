pub enum ToolAvailability {
    Available,
    Unavailable { reason: ToolUnavailableReason },
}

pub enum ToolUnavailableReason {
    FeatureDisabled,
    UnsupportedSession,
    AuthenticationRequired,
    ServerConversationRequired,
    CodebaseNotIndexed,
    NoActiveCommand,
    NoDocuments,
    NoSkills,
    NoComputerUseSession,
    NoActiveAgents,
    PermissionDisabled,
    /// This tool requires a Warp backend or Warp Drive connection that is not
    /// available in local-only mode. Suppressing it prevents accidental egress
    /// in violation of the zero-egress privacy rule.
    LocalOnlyViolation,
}
