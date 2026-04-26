use harness_contracts::Message;

pub trait TokenCounter: Send + Sync + 'static {
    fn count_tokens(&self, text: &str, model: &str) -> usize;
    fn count_messages(&self, messages: &[Message], model: &str) -> usize;

    fn count_image(&self, _image: &ImageMeta, _model: &str) -> Option<usize> {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageMeta {
    pub width: u32,
    pub height: u32,
    pub mime: String,
    pub detail: ImageDetail,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageDetail {
    Low,
    High,
    Auto,
}
