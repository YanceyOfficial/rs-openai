use bon::Builder;
use serde::{Deserialize, Serialize};

#[derive(Builder, Debug, Serialize, Clone)]
pub struct CreateConversationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Vec<crate::interfaces::responses::ResponseInputItem>>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct Conversation {
    pub id: String,
    pub object: String,
    pub created_at: i64,
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Builder, Debug, Serialize, Clone)]
pub struct UpdateConversationRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<std::collections::HashMap<String, String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct DeleteConversationResponse {
    pub id: String,
    pub object: String,
    pub deleted: bool,
}

// Items sub-resource, appended here (matching the established colocation precedent
// for a small module that isn't already split into a directory).
//
// `ConversationItem` (the response shape for a single item, both standalone and
// inside a list) is confirmed via source to be a 27-variant tagged union pulling in
// over a dozen Response item types this crate has never modeled (LocalShellCall,
// Program/ProgramOutput, ApplyPatchToolCall, FunctionShellToolCall,
// ResponseCompactionItem, ResponseCustomToolCall, etc. -- an entire newer
// agentic-tooling surface). Kept as raw `serde_json::Value`, matching this crate's
// established pattern for genuinely oversized/complex unions (Audit Log details,
// Evals' `data_source`) -- fully typing it would mean building a large new slice of
// Responses first, which is out of scope for closing this one gap.
//
// The CREATE side is unaffected: `items` continues to reuse the existing, already-
// scoped-down `ResponseInputItem` (see `CreateConversationRequest.items` above).

#[derive(Builder, Debug, Serialize, Clone)]
pub struct AddConversationItemsRequest {
    pub items: Vec<crate::interfaces::responses::ResponseInputItem>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub include: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct ConversationItemList {
    pub data: Vec<serde_json::Value>,
    pub first_id: String,
    pub has_more: bool,
    pub last_id: String,
    pub object: String,
}

/// `include`/`after`/`limit`/`order` are all query params -- `include` (an array
/// field) needs `to_query_pairs()` rather than the generic `get()` path, since
/// `serde_urlencoded` (this crate's query backend) cannot serialize a `Vec<String>`
/// struct field (confirmed in the Usage slice). Same fix reused here via the
/// already-existing `client.get_with_pairs()`.
#[derive(Builder, Debug, Clone, Default)]
pub struct ListConversationItemsRequest {
    #[builder(into)]
    pub after: Option<String>,
    pub include: Option<Vec<String>>,
    pub limit: Option<u32>,
    pub order: Option<crate::interfaces::chat::SortOrder>,
}

impl ListConversationItemsRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        if let Some(v) = &self.after {
            pairs.push(("after".to_string(), v.clone()));
        }
        if let Some(v) = self.limit {
            pairs.push(("limit".to_string(), v.to_string()));
        }
        if let Some(v) = &self.order {
            pairs.push(("order".to_string(), v.to_string()));
        }
        if let Some(values) = &self.include {
            for v in values {
                pairs.push(("include".to_string(), v.clone()));
            }
        }
        pairs
    }
}

#[derive(Builder, Debug, Clone, Default)]
pub struct RetrieveConversationItemRequest {
    pub include: Option<Vec<String>>,
}

impl RetrieveConversationItemRequest {
    pub(crate) fn to_query_pairs(&self) -> Vec<(String, String)> {
        let mut pairs = Vec::new();
        if let Some(values) = &self.include {
            for v in values {
                pairs.push(("include".to_string(), v.clone()));
            }
        }
        pairs
    }
}
