use futures::future::BoxFuture;

pub struct LexicalSearchRequest {
    pub query: String,
}

#[derive(Debug)]
pub struct LexicalSearchResult {
    pub files: Vec<String>,
}

#[derive(Debug)]
pub struct LexicalSearchError;

pub trait CodeSearchFallback {
    fn search(
        &self,
        request: LexicalSearchRequest,
    ) -> BoxFuture<'static, Result<LexicalSearchResult, LexicalSearchError>>;
}
