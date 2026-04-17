use super::*;

#[derive(Debug)]
pub struct MessageStream {
    request_id: Option<String>,
    response: reqwest::Response,
    parser: SseParser,
    pending: VecDeque<StreamEvent>,
    done: bool,
    request: MessageRequest,
    prompt_cache: Option<PromptCache>,
    latest_usage: Option<Usage>,
    usage_recorded: bool,
    last_prompt_cache_record: Arc<Mutex<Option<PromptCacheRecord>>>,
}

impl MessageStream {
    pub(super) fn new(
        response: reqwest::Response,
        request: MessageRequest,
        prompt_cache: Option<PromptCache>,
        last_prompt_cache_record: Arc<Mutex<Option<PromptCacheRecord>>>,
    ) -> Self {
        let request_id = request_id_from_headers(response.headers());
        let parser = SseParser::new().with_context("Anthropic", request.model.clone());
        Self {
            request_id,
            response,
            parser,
            pending: VecDeque::new(),
            done: false,
            request,
            prompt_cache,
            latest_usage: None,
            usage_recorded: false,
            last_prompt_cache_record,
        }
    }

    #[must_use]
    pub fn request_id(&self) -> Option<&str> {
        self.request_id.as_deref()
    }

    pub async fn next_event(&mut self) -> Result<Option<StreamEvent>, ApiError> {
        loop {
            if let Some(event) = self.pending.pop_front() {
                self.observe_event(&event);
                return Ok(Some(event));
            }

            if self.done {
                let remaining = self.parser.finish()?;
                self.pending.extend(remaining);
                if let Some(event) = self.pending.pop_front() {
                    return Ok(Some(event));
                }
                return Ok(None);
            }

            match self.response.chunk().await? {
                Some(chunk) => {
                    self.pending.extend(self.parser.push(&chunk)?);
                }
                None => {
                    self.done = true;
                }
            }
        }
    }

    fn observe_event(&mut self, event: &StreamEvent) {
        match event {
            StreamEvent::MessageDelta(MessageDeltaEvent { usage, .. }) => {
                self.latest_usage = Some(usage.clone());
            }
            StreamEvent::MessageStop(_) if !self.usage_recorded => {
                if let (Some(prompt_cache), Some(usage)) =
                    (&self.prompt_cache, self.latest_usage.as_ref())
                {
                    let record = prompt_cache.record_usage(&self.request, usage);
                    *self
                        .last_prompt_cache_record
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(record);
                }
                self.usage_recorded = true;
            }
            _ => {}
        }
    }
}
