use crate::config::LlmConfig;
use crate::ai::dto::{AiScenario, AiSuggestion};

pub enum LlmBackend {
    OpenAi { api_key: String, model: String },
    AzureOpenAi { endpoint: String, api_key: String, deployment: String },
    Ollama { url: String, model: String },
    Mock,
}

impl LlmBackend {
    pub fn from_config(cfg: &LlmConfig) -> Self {
        let backend = cfg.backend.as_str();
        match backend {
            "openai" => LlmBackend::OpenAi {
                api_key: cfg.api_key.clone().unwrap_or_default(),
                model: cfg.model.clone().unwrap_or_else(|| "gpt-4".to_string()),
            },
            "azure_openai" => LlmBackend::AzureOpenAi {
                endpoint: cfg.endpoint.clone().unwrap_or_default(),
                api_key: cfg.api_key.clone().unwrap_or_default(),
                deployment: cfg.model.clone().unwrap_or_default(),
            },
            "ollama" => LlmBackend::Ollama {
                url: cfg.endpoint.clone().unwrap_or_else(|| "http://localhost:11434".to_string()),
                model: cfg.model.clone().unwrap_or_else(|| "llama3".to_string()),
            },
            _ => LlmBackend::Mock,
        }
    }

    pub async fn chat(&self, _prompt: &str) -> anyhow::Result<String> {
        match self {
            LlmBackend::Mock => Ok("Mock LLM response".to_string()),
            LlmBackend::OpenAi { .. } => Ok("OpenAI response (not yet implemented)".to_string()),
            LlmBackend::AzureOpenAi { .. } => Ok("Azure OpenAI response (not yet implemented)".to_string()),
            LlmBackend::Ollama { .. } => Ok("Ollama response (not yet implemented)".to_string()),
        }
    }

    pub fn is_available(&self) -> bool {
        match self {
            LlmBackend::Mock => true,
            LlmBackend::OpenAi { api_key, .. } => !api_key.is_empty(),
            LlmBackend::AzureOpenAi { api_key, .. } => !api_key.is_empty(),
            LlmBackend::Ollama { .. } => true,
        }
    }

    pub async fn suggest(&self, scenario: &AiScenario, _context: &serde_json::Value) -> anyhow::Result<Vec<AiSuggestion>> {
        let suggestions = match scenario {
            AiScenario::CapabilityDecomposition => vec![
                AiSuggestion {
                    entity_type: "business_capability".to_string(),
                    action: "decompose".to_string(),
                    data: serde_json::json!({
                        "parent": "客户管理",
                        "children": ["客户信息管理", "客户关系维护", "客户满意度分析"]
                    }),
                    confidence: 0.85,
                    reasoning: "基于业务架构最佳实践，客户管理能力可分解为三个子能力".to_string(),
                },
            ],
            AiScenario::ProcessDesign => vec![
                AiSuggestion {
                    entity_type: "business_process".to_string(),
                    action: "design".to_string(),
                    data: serde_json::json!({
                        "process_name": "客户入职流程",
                        "steps": ["接收申请", "审核资料", "创建账户", "发送通知"]
                    }),
                    confidence: 0.78,
                    reasoning: "根据行业标杆流程设计，客户入职包含4个关键步骤".to_string(),
                },
            ],
            AiScenario::GapAnalysis => vec![
                AiSuggestion {
                    entity_type: "gap".to_string(),
                    action: "identify".to_string(),
                    data: serde_json::json!({
                        "current": "手动审批",
                        "target": "自动审批",
                        "gap": "缺乏自动审批规则引擎"
                    }),
                    confidence: 0.72,
                    reasoning: "当前流程依赖人工审批，目标状态需要规则引擎支持自动化".to_string(),
                },
            ],
            AiScenario::RedundancyIdentification => vec![
                AiSuggestion {
                    entity_type: "redundancy".to_string(),
                    action: "identify".to_string(),
                    data: serde_json::json!({
                        "entities": ["客户信息录入A", "客户信息录入B"],
                        "overlap": "两个流程都包含客户基本信息录入"
                    }),
                    confidence: 0.68,
                    reasoning: "检测到两个流程存在重复的客户信息录入步骤，建议合并".to_string(),
                },
            ],
        };
        Ok(suggestions)
    }
}
