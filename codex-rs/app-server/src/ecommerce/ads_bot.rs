use std::collections::HashMap;
use std::time::Instant;

use serde_json::{json, Value};
use uuid::Uuid;

use super::agent_traits::make_step_result;
use super::agent_traits::StepOutput;
use super::knowledge_base::{KbEntry, KnowledgeBase};
use super::llm_client::LlmClient;
use super::types::EcommerceAgentSubmitParams;
use super::types::EcommerceAgentSubmitResponse;
use super::types::EcommerceAgentType;
use super::types::WorkflowContext;
use codex_app_server_protocol::JSONRPCErrorError;

// ---------------------------------------------------------------------------
// System prompts (exact copies from the Dify document)
// ---------------------------------------------------------------------------

const ADS_INPUT_PARSER_SYSTEM: &str = r#"# Role
你是 AdsBot 的需求解析模块。

# Task
解析用户的广告文案需求。

# Output Format（严格 JSON）
{"status": "complete","ad_type": "AMAZON_PPC | GOOGLE_ADS | META_ADS | TIKTOK_ADS | EDM_EMAIL | SOCIAL_MEDIA","product_info": {"product_name": "产品名称","key_features": ["卖点1", "卖点2"],"price": "价格或null","target_audience": "目标人群或null","usp": "独特卖点或null"},"target_market": "US","target_language": "en","campaign_goal": "AWARENESS | TRAFFIC | CONVERSION | RETARGETING","tone": "professional | casual | urgent | luxurious | playful","competitor_differentiation": "与竞品的差异点或null","budget_level": "low | medium | high | null","num_variants": 5}

# Rules
1. 如果信息不全，追问关键信息（至少需要：产品名称、广告类型、目标市场）
2. 如果用户没指定广告类型，追问
3. 默认生成 5 个变体
4. 使用中文与用户交流

# 追问模板
"为了生成最佳的广告文案，请告诉我：

1. **广告类型**：Amazon站内广告 / Google Ads / Facebook&Instagram / TikTok / 邮件营销 / 社交媒体帖子？
2. **产品信息**：产品名称和2-3个核心卖点
3. **目标市场**：美国/欧洲/日本/其他？

如果你有竞品链接或想突出的差异化卖点，也请一并告诉我 😊""#;

const ADS_GENERATOR_SYSTEM: &str = r##"# Role
你是一位顶级的跨境电商广告文案专家，曾管理过累计数百万美金的广告预算。你深谙各广告平台的规则和最佳实践，能写出高点击率、高转化率的广告文案。

# Input
- 广告需求：{{ad_requirements}}
- SEO关键词数据：{{kb_seo_keywords}}
- 营销表达库：{{kb_multilingual_expressions}}
- 广告平台规则：{{kb_ads_rules}}
- 违禁词库：{{kb_compliance_rules}}

# 各广告类型的生成规范

## AMAZON_PPC — Amazon站内广告

### Sponsored Products - 标题广告
规范：
- 字符限制：50 characters
- 必须包含产品核心关键词
- 不得使用：价格/折扣信息、"#1"/"Best"等主观最高级、特殊字符
- 生成数量：5 个变体

变体策略：
变体1 - 功能驱动：突出最强功能
变体2 - 场景驱动：突出使用场景
变体3 - 数据驱动：用数字吸引
变体4 - 痛点驱动：解决问题
变体5 - 品质驱动：强调品质

### Sponsored Brands - 品牌广告标题
规范：
- 标题字符限制：50 characters
- 品牌标语要求：简洁、体现品牌价值
- 可包含品牌名
- 生成：品牌标语 + 5个广告标题变体

### Sponsored Display - 展示广告
规范：
- 标题：50 characters
- 描述：150 characters（部分广告位支持）
- 策略：更侧重于品牌认知和再营销

## GOOGLE_ADS — Google搜索/展示广告

### 搜索广告 (RSA - Responsive Search Ads)
规范：
- 标题：最多15个，每个 ≤ 30 characters
- 描述：最多4个，每个 ≤ 90 characters
- 建议至少提供：8个标题 + 3个描述
- 标题中要包含搜索关键词
- 描述中要包含 CTA (Call to Action)

### 购物广告（Google Shopping）
规范：
- 产品标题：≤ 150 characters（建议70-100）
- 产品描述：≤ 5000 characters（建议500-1000）
- 标题结构：Brand + Product Type + Key Attributes

## META_ADS — Facebook & Instagram 广告

### 信息流广告
规范：
- 广告标题：≤ 40 characters（超过会被截断）
- 正文文案：推荐 125 characters（桌面端），建议短文案和长文案各一版
- 描述：≤ 30 characters
- CTA按钮：Shop Now / Learn More / Sign Up / Get Offer

### Instagram Story/Reels 广告
规范：
- 文案极简：1-2句话
- 强CTA
- 配合视觉内容的文案建议

## TIKTOK_ADS — TikTok 广告
规范：
- 广告文案：≤ 100 characters（英文）/ ≤ 50 characters（中文）
- 风格：原生感、非广告感、年轻化
- 避免过于正式的商业语言
- 善用 emoji 和网络流行语

## EDM_EMAIL — 邮件营销
规范：
生成完整邮件内容包括主题行、预览文本、邮件正文

## SOCIAL_MEDIA — 社交媒体帖子
同时生成 Instagram、Facebook、Twitter/X、Pinterest 帖子文案

# 多语言广告文案规则
如果目标市场非英语：
1. 直接用目标语言撰写，不是翻译
2. 使用当地消费者的表达习惯
3. 广告平台违禁词在各语言中可能不同，需要分别检查
4. 文化敏感性检查（避免在特定市场引起不适的表达）

# Output Format

## 📢 广告文案生成结果

### 广告类型：[类型名称]
### 目标市场：[市场]
### 产品：[产品名称]

---

[根据广告类型输出对应格式的文案]

每个变体标注：
- 📊 **策略说明：** 这个变体为什么这样写
- 🎯 **适用场景：** 在什么情况下使用这个变体效果最好
- ⚠️ **注意事项：** 使用时需要注意什么

---

### 💡 投放建议
- 建议测试优先级：变体X > 变体X > 变体X
- 建议受众定向：...
- 建议出价策略：...
- 预期 CTR 参考范围：X% - X%

### ⚠️ 合规检查结果
[自动检查文案中是否有违反平台广告政策的内容]

# Constraints
1. 严格遵守各广告平台的字符限制
2. 不使用竞品品牌名（Google Ads 可以在特定策略下使用，但默认不用）
3. 不做虚假声明/未经验证的效果承诺
4. 每个变体必须有实质差异，不要只是换个词
5. 输出时分析说明用中文，广告文案本身使用目标语言"##;

const ADS_COMPLIANCE_CHECK_SYSTEM: &str = r#"# Role
你是广告合规审核员。

# Task
检查生成的广告文案是否符合目标广告平台的政策。

# 检查项目

## 1. 平台通用禁止内容
- 虚假/误导性声明
- 未经授权的健康声明
- 歧视性内容
- 版权/商标侵权
- 成人内容/暴力内容
- 政治/宗教敏感内容

## 2. Amazon PPC 特定规则
- 不得包含价格/折扣信息
- 不得使用"Best Seller"/#1等声明
- 不得提及Amazon品牌名
- 不得使用客户评价内容作为广告文本

## 3. Google Ads 特定规则
- 标题不得全大写
- 不得使用过多感叹号
- 不得使用"Click Here"作为唯一CTA
- 落地页必须与广告内容相关

## 4. Meta Ads 特定规则
- 图片中文字不超过20%面积
- 不得使用"before/after"对比图（部分品类）
- 不得暗示用户的个人属性
- 不得使用恐吓/负面情绪过度的表达

## 5. TikTok Ads 特定规则
- 不得使用"TikTok"品牌名
- 不得鼓励危险行为
- 不得使用他人肖像（未经授权）

# Output Format

### ✅ 广告合规检查

| 变体 | 平台规则 | 结果 | 问题详情 | 修改建议 |
|------|---------|------|---------|---------|
| 变体1 | [规则名] | 🟢通过/🔴违规 | [详情] | [建议] |

**整体合规评估：** 🟢 全部通过 / 🟡 部分需修改 / 🔴 存在严重违规"#;

// ---------------------------------------------------------------------------
// AdsBot struct
// ---------------------------------------------------------------------------

pub(crate) struct AdsBot {
    cached_kb: Vec<KbEntry>,
}

impl AdsBot {
    pub(crate) fn new(kb: &KnowledgeBase) -> Self {
        let mut cached = Vec::new();
        let kb_names = [
            "kb-02-seo-keywords",
            "kb-04-terminology",
            "kb-06-product-catalog",
            "kb-ads-01-platform-rules",
        ];
        for name in &kb_names {
            cached.extend(kb.query(name, None, None, None, 100));
        }
        Self { cached_kb: cached }
    }

    pub(crate) async fn run(
        &self,
        params: &EcommerceAgentSubmitParams,
        llm: &LlmClient,
    ) -> EcommerceAgentSubmitResponse {
        let request_id = Uuid::new_v4().to_string();
        let mut ctx: WorkflowContext = HashMap::new();
        let mut steps = Vec::new();

        ctx.insert("user_input".to_string(), json!(params.user_input));
        if let Some(ref platform) = params.platform {
            ctx.insert("platform".to_string(), json!(platform));
        }
        if let Some(ref market) = params.market {
            ctx.insert("market".to_string(), json!(market));
        }
        if let Some(ref context_map) = params.context {
            for (k, v) in context_map {
                ctx.insert(k.clone(), json!(v));
            }
        }

        // Step 1: Needs parse
        {
            let start = Instant::now();
            let step = match step_ads_input_parser(&mut ctx, llm).await {
                Ok(s) => s,
                Err(e) => {
                    return build_partial_response(
                        request_id,
                        params.agent_type,
                        steps,
                        format!("AdsInputParser failed: {:?}", e),
                    );
                }
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Step 2: KB retrieve (non-LLM)
        {
            let start = Instant::now();
            let step = step_ads_kb_retrieval(&mut ctx, &self.cached_kb);
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Step 3: Ad generate
        {
            let start = Instant::now();
            let step = match step_ads_generator(&mut ctx, llm).await {
                Ok(s) => s,
                Err(e) => {
                    return build_partial_response(
                        request_id,
                        params.agent_type,
                        steps,
                        format!("AdsGenerator failed: {:?}", e),
                    );
                }
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Step 4: Compliance check
        {
            let start = Instant::now();
            let step = match step_ads_compliance_check(&mut ctx, llm).await {
                Ok(s) => s,
                Err(e) => {
                    return build_partial_response(
                        request_id,
                        params.agent_type,
                        steps,
                        format!("AdsComplianceCheck failed: {:?}", e),
                    );
                }
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Step 5: Output assemble (non-LLM)
        {
            let start = Instant::now();
            let step = step_output_assembler(&mut ctx);
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        let markdown = ctx_str(&ctx, "final_markdown");

        EcommerceAgentSubmitResponse {
            request_id,
            agent_type: params.agent_type,
            status: "completed".to_string(),
            result: Some(json!({
                "steps_completed": 5,
                "agent": "AdsBot",
            })),
            output_markdown: Some(markdown),
            intermediate_steps: steps,
            error: None,
        }
    }
}

// ---------------------------------------------------------------------------
// Step implementations
// ---------------------------------------------------------------------------

async fn step_ads_input_parser(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let user_input = ctx_str(ctx, "user_input");

    let user_message = format!("用户需求: {}", user_input);
    let response = llm
        .chat("gpt-4o-mini", ADS_INPUT_PARSER_SYSTEM, &user_message)
        .await?;

    let parsed: Value = serde_json::from_str(&response).unwrap_or_else(|_| {
        json!({
            "status": "complete",
            "ad_type": "AMAZON_PPC",
            "product_info": {
                "product_name": user_input.chars().take(50).collect::<String>(),
                "key_features": ["quality", "value"],
                "price": null,
                "target_audience": null,
                "usp": null
            },
            "target_market": "US",
            "target_language": "en",
            "campaign_goal": "CONVERSION",
            "tone": "professional",
            "competitor_differentiation": null,
            "budget_level": null,
            "num_variants": 5
        })
    });

    ctx.insert("ads_input_parser".to_string(), parsed.clone());

    Ok(StepOutput {
        step_name: "ads_input_parser".to_string(),
        model: "gpt-4o-mini".to_string(),
        output: parsed,
    })
}

fn step_ads_kb_retrieval(ctx: &mut WorkflowContext, cached_kb: &[KbEntry]) -> StepOutput {
    let _user_input = ctx_str(ctx, "user_input");

    let kb_context: String = cached_kb
        .iter()
        .map(|e| e.content.clone())
        .collect::<Vec<_>>()
        .join("\n---\n");

    ctx.insert("ads_kb_results".to_string(), json!(kb_context));

    StepOutput {
        step_name: "ads_kb_retrieval".to_string(),
        model: "".to_string(),
        output: json!({
            "kb_queried": ["kb-02-seo-keywords", "kb-04-terminology", "kb-06-product-catalog", "kb-ads-01"],
            "entries_loaded": cached_kb.len(),
            "context_length": kb_context.len(),
        }),
    }
}

async fn step_ads_generator(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let ad_requirements = ctx
        .get("ads_input_parser")
        .cloned()
        .unwrap_or(json!({}));
    let kb_results = ctx_str(ctx, "ads_kb_results");

    let system_prompt = ADS_GENERATOR_SYSTEM
        .replace("{{ad_requirements}}", &ad_requirements.to_string())
        .replace("{{kb_seo_keywords}}", &kb_results)
        .replace("{{kb_multilingual_expressions}}", &kb_results)
        .replace("{{kb_ads_rules}}", &kb_results)
        .replace("{{kb_compliance_rules}}", &kb_results);

    let ad_type = ad_requirements
        .get("ad_type")
        .and_then(|v| v.as_str())
        .unwrap_or("AMAZON_PPC");
    let product_name = ad_requirements
        .get("product_info")
        .and_then(|pi| pi.get("product_name"))
        .and_then(|v| v.as_str())
        .unwrap_or("Product");
    let target_market = ad_requirements
        .get("target_market")
        .and_then(|v| v.as_str())
        .unwrap_or("US");

    let user_message = format!(
        "请为以下产品生成 {} 广告文案:\n\n产品: {}\n目标市场: {}\n\n广告需求详情: {}",
        ad_type, product_name, target_market, ad_requirements,
    );

    let response = llm
        .chat("claude-3-5-sonnet-20241022", &system_prompt, &user_message)
        .await?;

    let output = json!({
        "ads_copy": response,
    });

    ctx.insert("ads_generator".to_string(), json!(response));

    Ok(StepOutput {
        step_name: "ads_generator".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        output,
    })
}

async fn step_ads_compliance_check(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let ads_copy = ctx_str(ctx, "ads_generator");
    let ad_requirements = ctx
        .get("ads_input_parser")
        .cloned()
        .unwrap_or(json!({}));
    let ad_type = ad_requirements
        .get("ad_type")
        .and_then(|v| v.as_str())
        .unwrap_or("AMAZON_PPC");
    let target_market = ad_requirements
        .get("target_market")
        .and_then(|v| v.as_str())
        .unwrap_or("US");

    let user_message = format!(
        "广告文案:\n{}\n\n广告类型: {}\n目标市场: {}",
        ads_copy, ad_type, target_market,
    );

    let response = llm
        .chat("gpt-4o-mini", ADS_COMPLIANCE_CHECK_SYSTEM, &user_message)
        .await?;

    let output = json!({
        "compliance_report": response,
    });

    ctx.insert("ads_compliance_check".to_string(), json!(response));

    Ok(StepOutput {
        step_name: "ads_compliance_check".to_string(),
        model: "gpt-4o-mini".to_string(),
        output,
    })
}

fn step_output_assembler(ctx: &mut WorkflowContext) -> StepOutput {
    let ads_copy = ctx_str(ctx, "ads_generator");
    let compliance = ctx_str(ctx, "ads_compliance_check");

    let markdown = format!(
        r#"# 📢 AdsBot — 广告文案生成完成

---

{ads_copy}

---

{compliance}

---

## 📌 使用建议

1. **A/B 测试：** 建议同时投放 2-3 个变体，运行 7 天后根据 CTR 和转化率保留最优变体
2. **持续优化：** 每 2 周更换一次广告文案，避免用户疲劳
3. **数据回传：** 记录每个变体的表现数据，下次生成时告诉我，我会学习优化

💬 **需要调整？** 你可以说：
- "变体3的语气再激进一些"
- "帮我加一个突出价格优势的版本"
- "把这些文案翻译成日语版"
- "生成配合这些文案的视频脚本"
"#,
        ads_copy = ads_copy,
        compliance = compliance,
    );

    ctx.insert("final_markdown".to_string(), json!(markdown));

    StepOutput {
        step_name: "output_assembler".to_string(),
        model: "".to_string(),
        output: json!({"output_length": markdown.len()}),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ctx_str<'a>(ctx: &'a WorkflowContext, key: &str) -> String {
    ctx.get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string()
}

fn build_partial_response(
    request_id: String,
    agent_type: EcommerceAgentType,
    intermediate_steps: Vec<super::types::EcommerceAgentStepResult>,
    error: String,
) -> EcommerceAgentSubmitResponse {
    EcommerceAgentSubmitResponse {
        request_id,
        agent_type,
        status: "partial".to_string(),
        result: None,
        output_markdown: None,
        intermediate_steps,
        error: Some(error),
    }
}
