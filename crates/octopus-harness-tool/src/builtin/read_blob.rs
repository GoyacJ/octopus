use async_trait::async_trait;
use futures::{stream, StreamExt};
use harness_contracts::{
    BlobRef, BlobStore, DecisionScope, PermissionSubject, ToolCapability, ToolDescriptor,
    ToolError, ToolGroup, ToolResult,
};
use harness_permission::PermissionCheck;
use serde_json::{json, Value};

use crate::{Tool, ToolContext, ToolEvent, ToolStream, ValidationError};

#[derive(Clone)]
pub struct ReadBlobTool {
    descriptor: ToolDescriptor,
}

impl Default for ReadBlobTool {
    fn default() -> Self {
        Self {
            descriptor: super::descriptor(
                "ReadBlob",
                "Read blob",
                "Read a previously offloaded tool result blob.",
                ToolGroup::Meta,
                true,
                true,
                false,
                64_000,
                vec![ToolCapability::BlobReader],
                super::object_schema(
                    &["blob_ref"],
                    json!({
                        "blob_ref": { "type": "object" }
                    }),
                ),
            ),
        }
    }
}

#[async_trait]
impl Tool for ReadBlobTool {
    fn descriptor(&self) -> &ToolDescriptor {
        &self.descriptor
    }

    async fn validate(&self, input: &Value, _ctx: &ToolContext) -> Result<(), ValidationError> {
        blob_ref(input)?;
        Ok(())
    }

    async fn check_permission(&self, input: &Value, _ctx: &ToolContext) -> PermissionCheck {
        PermissionCheck::AskUser {
            subject: PermissionSubject::ToolInvocation {
                tool: self.descriptor.name.clone(),
                input: input.clone(),
            },
            scope: DecisionScope::ToolName(self.descriptor.name.clone()),
        }
    }

    async fn execute(&self, input: Value, ctx: ToolContext) -> Result<ToolStream, ToolError> {
        let blob_ref = blob_ref(&input).map_err(validation_error)?;
        let store = ctx.capability::<dyn BlobStore>(ToolCapability::BlobReader)?;
        let bytes = store
            .get(ctx.tenant_id, &blob_ref)
            .await
            .map_err(|error| ToolError::Message(error.to_string()))?
            .collect::<Vec<_>>()
            .await
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let text =
            String::from_utf8(bytes).map_err(|error| ToolError::Message(error.to_string()))?;
        Ok(Box::pin(stream::iter([ToolEvent::Final(
            ToolResult::Text(text),
        )])))
    }
}

fn validation_error(error: ValidationError) -> ToolError {
    ToolError::Validation(error.to_string())
}

fn blob_ref(input: &Value) -> Result<BlobRef, ValidationError> {
    serde_json::from_value(
        input
            .get("blob_ref")
            .cloned()
            .ok_or_else(|| ValidationError::from("blob_ref is required"))?,
    )
    .map_err(|error| ValidationError::from(error.to_string()))
}
