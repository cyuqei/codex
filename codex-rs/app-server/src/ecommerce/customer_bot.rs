use std::collections::HashMap;
use std::time::Instant;

use serde_json::{json, Value};
use uuid::Uuid;

use super::agent_traits::make_step_result;
use super::agent_traits::StepOutput;
use super::conversation_store::ConversationStore;
use super::knowledge_base::KnowledgeBase;
use super::llm_client::LlmClient;
use super::types::EcommerceAgentSubmitParams;
use super::types::EcommerceAgentSubmitResponse;
use super::types::EcommerceAgentType;
use super::types::WorkflowContext;
use codex_app_server_protocol::JSONRPCErrorError;

// ---------------------------------------------------------------------------
// System prompts (exact copies from the Dify document)
// ---------------------------------------------------------------------------

const LANG_INTENT_DETECTOR_SYSTEM: &str = r#"# Role
你是 CustomerBot 的语言检测与意图分类模块。

# Task
1. 检测用户消息的语言
2. 判断用户的客服意图
3. 评估用户的情绪状态

# 语言检测
识别以下语言：
- en: English
- ja: 日本語
- de: Deutsch
- fr: Français
- es: Español
- it: Italiano
- ko: 한국어
- zh: 中文
- other: 其他语言

# 意图分类
## PRODUCT_INQUIRY — 产品咨询
用户询问产品的功能、规格、使用方法、兼容性等
关键词：how to use, does it work with, what size, 使い方, Funktion, compatible

## SHIPPING_INQUIRY — 物流咨询
用户询问发货时间、物流状态、配送范围等
关键词：when will it arrive, shipping, tracking, delivery, 配送, Lieferung, envío

## RETURN_REFUND — 退换货/退款
用户想要退货、换货、退款
关键词：return, refund, exchange, broken, damaged, wrong item, 返品, Rückgabe, devolución

## COMPLAINT — 投诉/不满
用户表达强烈不满、愤怒或威胁
关键词：terrible, worst, sue, report, fraud, never again, 最悪, Beschwerde

## ORDER_ISSUE — 订单问题
用户关于订单状态、取消订单、修改订单等
关键词：order status, cancel, modify, haven't received, 注文, Bestellung

## POSITIVE_FEEDBACK — 正面反馈
用户表达满意、感谢、好评
关键词：love it, great product, thank you, works perfectly, ありがとう, danke

## OTHER — 其他
不属于以上任何类别

# 情绪检测
- positive: 积极/满意
- neutral: 中性/正常
- negative: 消极/不满
- angry: 愤怒/激动
- urgent: 紧急/焦虑

# Output Format（严格 JSON）
{"language": "en","intent": "PRODUCT_INQUIRY","emotion": "neutral","emotion_score": 0.3,"summary": "用户询问产品是否兼容iPhone 15","needs_escalation": false,"escalation_reason": null}

# 自动升级人工的条件（needs_escalation = true）
1. emotion 为 angry 且 emotion_score > 0.8
2. intent 为 COMPLAINT
3. 用户消息中提到法律/诉讼/投诉到平台
4. 用户明确要求与人工客服对话
5. 涉及安全问题（产品导致人身伤害等）"#;

const CS_RESPONSE_GENERATOR_SYSTEM: &str = r#"# Role
你是 {{brand_name}} 品牌的专属客服代表。你专业、耐心、友好，始终以解决客户问题为第一目标。

# Core Identity
- 品牌名称：{{brand_name}}
- 品牌调性：{{brand_tone}}（专业/友好/高端/年轻活力，由商家配置）
- 服务承诺：{{service_promise}}

# 核心规则（绝对不可违反）

## 规则1：语言匹配
客户用什么语言发消息，你就用什么语言回复。绝不切换语言。
例外：如果客户用中文发消息但询问的是面向海外客户的问题，仍然用中文回复。

## 规则2：情绪适配
- neutral/positive → 正常专业语气
- negative → 先表达理解和歉意，再解决问题
- angry → 先充分安抚（不要辩解），表达重视，再提供解决方案
- urgent → 表达理解紧迫性，快速给出解决方案

## 规则3：信息来源
- 仅使用知识库中提供的信息回复
- 如果知识库中没有对应答案，诚实告知客户需要进一步确认
- 绝不编造信息（错误信息会导致法律纠纷）
- 绝不承诺超出政策范围的赔偿/退款

## 规则4：隐私保护
- 不要在回复中暴露客户的个人信息
- 不要透露公司内部流程/成本信息
- 订单号等敏感信息仅在客户主动提供时引用

# 各意图的回复策略

## PRODUCT_INQUIRY — 产品咨询
1. 从知识库 KB-CS-02 检索产品相关信息
2. 清晰回答客户问题
3. 如果问题涉及多个方面，分点回答
4. 在回答末尾推荐相关产品或配件（如适用）
5. 如果知识库中没有该产品信息 → "非常感谢您的咨询。关于这个问题，我需要与产品团队确认后给您回复。请允许我在 24 小时内通过站内信回复您。"

## SHIPPING_INQUIRY — 物流咨询
1. 从知识库 KB-CS-03 检索物流政策
2. 提供该站点/市场的标准配送时间
3. 如果客户提供了订单号，告知如何追踪（但不要编造物流状态）
4. 如果发货延迟 → 致歉 + 说明原因 + 预计到达时间
5. FBA vs FBM 的回复差异：
   - FBA："您的订单由 Amazon 物流配送，您可以在订单详情页查看最新物流状态。"
   - FBM："我们已在 X 小时内发货，物流单号为 XXX，您可以在 [物流网站] 追踪。"

## RETURN_REFUND — 退换货/退款
1. 从知识库 KB-CS-04 检索退换货政策
2. 先表示理解和歉意
3. 询问退货原因（如果客户未说明）：产品质量问题 / 尺寸颜色不合适 / 不符合描述 / 不想要了 / 运输损坏
4. 根据退货原因和政策，给出对应解决方案：
   - 质量问题/运输损坏 → 优先提供全额退款或重新发货（免退回）
   - 尺寸不合适 → 提供换货流程
   - 不想要了 → 告知退货流程和可能的费用
5. 如果金额超过阈值 → 告知客户需要升级处理

## ORDER_ISSUE — 订单问题
1. 询问订单号（如果客户未提供）
2. 常见问题回复：
   - 取消订单："If your order hasn't shipped yet, we can help you cancel it. Could you please provide your order number?"
   - 修改订单："Unfortunately, we cannot modify orders once placed. However, [替代方案]"
   - 未收到货："We understand your concern. [根据配送时效判断是否正常] + [下一步操作]"

## POSITIVE_FEEDBACK — 正面反馈
1. 真诚感谢客户
2. 邀请客户留下评价（但不要过于直接/强迫）
3. 推荐客户关注品牌/店铺获取新品信息
4. 语气模板（英文）："Thank you so much for your kind words! We're thrilled to hear you love the product. If you have a moment, we'd truly appreciate a review — it helps other customers discover [brand_name]. Thank you for choosing us! 🙏"

# 各语言回复模板

## English
- 开头：Dear Customer, / Hello, / Thank you for reaching out.
- 结尾：If you have any other questions, please don't hesitate to ask. We're here to help! / Best regards, [Brand Name] Customer Service

## 日本語
- 开头：いつもご愛顧いただきありがとうございます。/ お問い合わせいただきありがとうございます。
- 结尾：他にご不明な点がございましたら、お気軽にお問い合わせください。今後ともよろしくお願いいたします。/ [Brand Name]カスタマーサービス

## Deutsch
- 开头：Vielen Dank für Ihre Nachricht. / Sehr geehrte/r Kunde/Kundin,
- 结尾：Wenn Sie weitere Fragen haben, stehen wir Ihnen gerne zur Verfügung. / Mit freundlichen Grüßen, [Brand Name] Kundenservice

## Français
- 开头：Merci de nous avoir contactés. / Cher(e) client(e),
- 结尾：N'hésitez pas à nous contacter si vous avez d'autres questions. / Cordialement, Service client [Brand Name]

## Español
- 开头：Gracias por contactarnos. / Estimado/a cliente,
- 结尾：No dude en contactarnos si tiene alguna otra pregunta. / Atentamente, Servicio al cliente de [Brand Name]

## Italiano
- 开头：Grazie per averci contattato. / Gentile cliente,
- 结尾：Non esiti a contattarci per qualsiasi altra domanda. / Cordiali saluti, Servizio clienti [Brand Name]

## 한국어
- 开头：문의해 주셔서 감사합니다. / 안녕하세요, 고객님.
- 结尾：추가 문의 사항이 있으시면 언제든지 연락 주세요. / 감사합니다. [Brand Name] 고객센터

# Output Format
直接输出回复内容，不需要任何元数据或 JSON 包装。
回复应该是客户可以直接阅读的最终版本。

# Constraints
1. 回复长度控制在 50-200 词（根据问题复杂度调整）
2. 不使用 Markdown 格式（客户看的是纯文本消息）
3. 可以适当使用 emoji，但不要过多（1-2个即可）
4. 不要生成"Dear Sir/Madam"这种过于老式的称呼
5. 不要在回复中提及"AI"/"机器人"/"自动回复"——让客户感觉是真人在服务"#;

const ESCALATION_HANDLER_SYSTEM: &str = r#"⚠️ 此对话需要人工介入

升级原因：{{escalation_reason}}
客户语言：{{language}}
客户情绪：{{emotion}}
问题摘要：{{summary}}

## 给客户的回复（已自动发送）：

English: "Thank you for your patience. I understand this is important to you, and I want to make sure you get the best assistance. I'm connecting you with our senior support team who will be able to help you further. You can expect a response within 2-4 hours. We sincerely apologize for any inconvenience."

日本語: "お待たせして申し訳ございません。お客様のお気持ちを十分に理解しております。より適切な対応をさせていただくため、担当チームにお繋ぎいたします。2〜4時間以内にご連絡差し上げます。ご不便をおかけして誠に申し訳ございません。"

Deutsch: "Vielen Dank für Ihre Geduld. Ich verstehe, dass dies wichtig für Sie ist. Um Ihnen bestmöglich zu helfen, leite ich Ihr Anliegen an unser spezialisiertes Support-Team weiter. Sie erhalten innerhalb von 2-4 Stunden eine Antwort. Wir entschuldigen uns für die Unannehmlichkeiten."

Français: "Merci de votre patience. Je comprends que cette situation est importante pour vous. Afin de vous offrir la meilleure assistance possible, je transfère votre demande à notre équipe de support spécialisée. Vous recevrez une réponse dans un délai de 2 à 4 heures. Nous nous excusons pour tout inconvénient."

Español: "Gracias por su paciencia. Entiendo que esto es importante para usted, y quiero asegurarme de que reciba la mejor asistencia. Le estoy conectando con nuestro equipo de soporte senior quien podrá ayudarle más. Puede esperar una respuesta dentro de 2-4 horas. Lamentamos cualquier inconveniente."

## 人工客服处理指引：
1. 优先处理此工单
2. 注意客户情绪状态，先安抚后处理
3. 处理完成后更新工单状态
"#;

// ---------------------------------------------------------------------------
// CustomerBot struct
// ---------------------------------------------------------------------------

pub(crate) struct CustomerBot {
    kb_cs_entries: std::collections::HashMap<String, Vec<super::knowledge_base::KbEntry>>,
    conversation_store: Option<ConversationStore>,
}

impl CustomerBot {
    pub(crate) fn new(kb: &KnowledgeBase) -> Self {
        let mut entries = std::collections::HashMap::new();
        let kb_names = [
            "kb-cs-01-faq",
            "kb-cs-02-product-info",
            "kb-cs-03-shipping",
            "kb-cs-04-returns",
            "kb-cs-05-scripts",
        ];
        for name in &kb_names {
            entries.insert(name.to_string(), kb.query(name, None, None, None, 100));
        }
        Self {
            kb_cs_entries: entries,
            conversation_store: None,
        }
    }

    /// Create a CustomerBot with multi-turn conversation support.
    pub(crate) fn with_conversation(kb: &KnowledgeBase, store: ConversationStore) -> Self {
        let mut entries = std::collections::HashMap::new();
        let kb_names = [
            "kb-cs-01-faq",
            "kb-cs-02-product-info",
            "kb-cs-03-shipping",
            "kb-cs-04-returns",
            "kb-cs-05-scripts",
        ];
        for name in &kb_names {
            entries.insert(name.to_string(), kb.query(name, None, None, None, 100));
        }
        Self {
            kb_cs_entries: entries,
            conversation_store: Some(store),
        }
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

        // Multi-turn conversation support
        if let Some(ref thread_id) = params.thread_id {
            // Save user message to conversation store
            if let Some(ref store) = self.conversation_store {
                store.add_message(thread_id, "user", &params.user_input);
            }

            // Inject conversation context into the context
            if let Some(ref store) = self.conversation_store {
                let conv_context = store.get_context(thread_id, /*max_turns*/ 10);
                ctx.insert("conversation_history".to_string(), json!(conv_context.unwrap_or_default()));
            } else {
                ctx.insert("conversation_history".to_string(), json!(""));
            }
        }

        // Step 1: Language/Intent detect
        {
            let start = Instant::now();
            let step = match step_lang_intent_detector(&mut ctx, llm).await {
                Ok(s) => s,
                Err(e) => {
                    return build_partial_response(
                        request_id,
                        params.agent_type,
                        steps,
                        format!("LangIntentDetector failed: {:?}", e),
                    );
                }
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Check escalation first
        let needs_escalation = ctx
            .get("lang_intent_detector")
            .and_then(|v| v.get("needs_escalation"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        if needs_escalation {
            let start = Instant::now();
            let step = step_escalation_handler(&mut ctx, params);
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));

            let markdown = ctx_str(&ctx, "final_markdown");

            // Save assistant response to conversation store (even for escalated)
            if let Some(ref thread_id) = params.thread_id {
                if let Some(ref store) = self.conversation_store {
                    store.add_message(thread_id, "assistant", &markdown);
                }
            }

            return EcommerceAgentSubmitResponse {
                request_id,
                agent_type: params.agent_type,
                status: "escalated".to_string(),
                result: Some(json!({
                    "escalated": true,
                    "steps_completed": steps.len(),
                    "agent": "CustomerBot",
                    "thread_id": params.thread_id,
                })),
                output_markdown: Some(markdown),
                intermediate_steps: steps,
                error: None,
            };
        }

        // Step 2: KB Retrieval (intent-aware)
        {
            let start = Instant::now();
            let step = step_kb_retrieval(&mut ctx, &self.kb_cs_entries);
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Step 3: Response generate
        {
            let start = Instant::now();
            let step = match step_cs_response_generator(&mut ctx, llm).await {
                Ok(s) => s,
                Err(e) => {
                    return build_partial_response(
                        request_id,
                        params.agent_type,
                        steps,
                        format!("CsResponseGenerator failed: {:?}", e),
                    );
                }
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Step 4: Output assemble (non-LLM)
        {
            let start = Instant::now();
            let step = step_output_assembler(&mut ctx, params);
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        let markdown = ctx_str(&ctx, "final_markdown");

        // Save assistant response to conversation store
        if let Some(ref thread_id) = params.thread_id {
            if let Some(ref store) = self.conversation_store {
                store.add_message(thread_id, "assistant", &markdown);
            }
        }

        EcommerceAgentSubmitResponse {
            request_id,
            agent_type: params.agent_type,
            status: "completed".to_string(),
            result: Some(json!({
                "steps_completed": steps.len(),
                "agent": "CustomerBot",
                "thread_id": params.thread_id,
                "turn_count": self.conversation_store.as_ref().and_then(|s| {
                    params.thread_id.as_ref().and_then(|tid| {
                        let guard = s.get_or_create(tid);
                        Some(guard.turn_count)
                    })
                }).unwrap_or(0),
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

async fn step_lang_intent_detector(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let user_input = ctx_str(ctx, "user_input");
    let conv_history = ctx
        .get("conversation_history")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let user_message = if !conv_history.is_empty() {
        format!("用户消息: {}\n\n{}\n\n请根据完整的对话历史判断用户语言、意图和情绪。", user_input, conv_history)
    } else {
        format!("用户消息: {}", user_input)
    };
    let response = llm
        .chat("gpt-4o-mini", LANG_INTENT_DETECTOR_SYSTEM, &user_message)
        .await?;

    let parsed: Value = serde_json::from_str(&response).unwrap_or_else(|_| {
        json!({
            "language": "en",
            "intent": "OTHER",
            "emotion": "neutral",
            "emotion_score": 0.0,
            "summary": user_input.chars().take(100).collect::<String>(),
            "needs_escalation": false,
            "escalation_reason": null
        })
    });

    ctx.insert("lang_intent_detector".to_string(), parsed.clone());

    Ok(StepOutput {
        step_name: "lang_intent_detector".to_string(),
        model: "gpt-4o-mini".to_string(),
        output: parsed,
    })
}

fn step_kb_retrieval(
    ctx: &mut WorkflowContext,
    kb_cs_entries: &std::collections::HashMap<String, Vec<super::knowledge_base::KbEntry>>,
) -> StepOutput {
    let intent: String = ctx
        .get("lang_intent_detector")
        .and_then(|v| v.get("intent"))
        .and_then(|v| v.as_str())
        .unwrap_or("OTHER")
        .to_string();

    let kb_name = match intent.as_str() {
        "PRODUCT_INQUIRY" => "kb-cs-02-product-info",
        "SHIPPING_INQUIRY" => "kb-cs-03-shipping",
        "RETURN_REFUND" => "kb-cs-04-returns",
        "ORDER_ISSUE" => "kb-cs-01-faq",
        "POSITIVE_FEEDBACK" => "kb-cs-05-scripts",
        _ => "kb-cs-01-faq",
    };

    let entries = kb_cs_entries.get(kb_name).cloned().unwrap_or_default();
    let combined: String = entries
        .iter()
        .map(|e| e.content.clone())
        .collect::<Vec<_>>()
        .join("\n---\n");

    ctx.insert("kb_cs_results".to_string(), json!(combined));

    StepOutput {
        step_name: "kb_retrieval".to_string(),
        model: "".to_string(),
        output: json!({
            "kb_name": kb_name,
            "intent": intent,
            "entries_found": entries.len(),
            "combined_length": combined.len(),
        }),
    }
}

async fn step_cs_response_generator(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let intent_detected = ctx
        .get("lang_intent_detector")
        .cloned()
        .unwrap_or(json!({}));
    let user_input = ctx_str(ctx, "user_input");
    let brand_name = ctx_str(ctx, "brand_name");
    let brand_name = if brand_name.is_empty() { "YourBrand" } else { &brand_name };

    let system_prompt = CS_RESPONSE_GENERATOR_SYSTEM
        .replace("{{brand_name}}", brand_name)
        .replace("{{brand_tone}}", "专业")
        .replace("{{service_promise}}", "客户满意是我们的首要目标");

    let kb_results = ctx_str(ctx, "kb_cs_results");
    let conv_history = ctx
        .get("conversation_history")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let user_message = if !conv_history.is_empty() {
        format!(
            "客户消息: {}\n\n检测结果: {}\n\n知识库信息: {}\n\n{}\n\n请根据完整的对话历史生成回复。",
            user_input,
            intent_detected,
            kb_results,
            conv_history,
        )
    } else {
        format!(
            "客户消息: {}\n\n检测结果: {}\n\n知识库信息: {}",
            user_input,
            intent_detected,
            kb_results,
        )
    };

    let response = llm
        .chat("gpt-4o", &system_prompt, &user_message)
        .await?;

    let output = json!({
        "response": response,
    });

    ctx.insert("cs_response".to_string(), json!(response));

    Ok(StepOutput {
        step_name: "cs_response_generator".to_string(),
        model: "gpt-4o".to_string(),
        output,
    })
}

fn step_escalation_handler(
    ctx: &mut WorkflowContext,
    _params: &EcommerceAgentSubmitParams,
) -> StepOutput {
    let detector = ctx.get("lang_intent_detector").cloned().unwrap_or(json!({}));
    let language = detector.get("language").and_then(|v| v.as_str()).unwrap_or("en");
    let summary = detector.get("summary").and_then(|v| v.as_str()).unwrap_or("");
    let reason = detector.get("escalation_reason").and_then(|v| v.as_str()).unwrap_or("");
    let emotion = detector.get("emotion").and_then(|v| v.as_str()).unwrap_or("neutral");

    let escalation_msg = match language {
        "ja" => "お待たせして申し訳ございません。お客様のお気持ちを十分に理解しております。より適切な対応をさせていただくため、担当チームにお繋ぎいたします。2〜4時間以内にご連絡差し上げます。ご不便をおかけして誠に申し訳ございません。",
        "de" => "Vielen Dank für Ihre Geduld. Ich verstehe, dass dies wichtig für Sie ist. Um Ihnen bestmöglich zu helfen, leite ich Ihr Anliegen an unser spezialisiertes Support-Team weiter. Sie erhalten innerhalb von 2-4 Stunden eine Antwort. Wir entschuldigen uns für die Unannehmlichkeiten.",
        "fr" => "Merci de votre patience. Je comprends que cette situation est importante pour vous. Afin de vous offrir la meilleure assistance possible, je transfère votre demande à notre équipe de support spécialisée. Vous recevrez une réponse dans un délai de 2 à 4 heures. Nous nous excusons pour tout inconvénient.",
        "es" => "Gracias por su paciencia. Entiendo que esto es importante para usted. Le estoy conectando con nuestro equipo de soporte senior quien podrá ayudarle más. Puede esperar una respuesta dentro de 2-4 horas. Lamentamos cualquier inconveniente.",
        _ => "Thank you for your patience. I understand this is important to you, and I want to make sure you get the best assistance. I'm connecting you with our senior support team who will be able to help you further. You can expect a response within 2-4 hours. We sincerely apologize for any inconvenience.",
    };

    let markdown = format!(
        r#"# CustomerBot — 已升级人工处理

**升级原因:** {reason}
**客户语言:** {language}
**客户情绪:** {emotion}
**问题摘要:** {summary}

---

**自动回复已发送:**

{escalation_msg}

---

**人工处理指引:**
1. 优先处理此工单
2. 注意客户情绪状态，先安抚后处理
3. 处理完成后更新工单状态
"#,
    );

    ctx.insert("escalation_message".to_string(), json!(escalation_msg));
    ctx.insert("final_markdown".to_string(), json!(markdown));

    StepOutput {
        step_name: "escalation_handler".to_string(),
        model: "".to_string(),
        output: json!({
            "escalated": true,
            "language": language,
        }),
    }
}

fn step_output_assembler(
    ctx: &mut WorkflowContext,
    _params: &EcommerceAgentSubmitParams,
) -> StepOutput {
    let response = ctx_str(ctx, "cs_response");
    let intent = ctx
        .get("lang_intent_detector")
        .and_then(|v| v.get("intent"))
        .and_then(|v| v.as_str())
        .unwrap_or("OTHER");

    let markdown = format!(
        r#"# CustomerBot — 客服回复

**意图分类:** {intent}

---

{response}
"#,
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
