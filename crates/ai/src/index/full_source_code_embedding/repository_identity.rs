use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FullSourceRepositoryIdentity {
    pub root_path: PathBuf,
}
