pub mod fs;

use super::executor::ToolRegistry;

/// In-process executors that need nothing but the filesystem and the working
/// directory. Every tool registered here is read-only and UI-free, which is
/// what makes them safe to run inside the harness loop without the client.
pub fn filesystem_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(fs::ReadFilesExecutor));
    registry.register(Box::new(fs::GlobExecutor));
    registry.register(Box::new(fs::GrepExecutor));
    registry
}
