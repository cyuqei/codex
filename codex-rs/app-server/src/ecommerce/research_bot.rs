use std::collections::HashMap;
use std::time::Instant;

use serde_json::{json, Value};
use uuid::Uuid;

use super::agent_traits::make_step_result;
use super::agent_traits::StepOutput;
use super::llm_client::LlmClient;
use super::types::EcommerceAgentSubmitParams;
use super::types::EcommerceAgentSubmitResponse;
use super::types::EcommerceAgentType;
use super::types::WorkflowContext;
use codex_app_server_protocol::JSONRPCErrorError;

// ---------------------------------------------------------------------------
// System prompts (exact copies from the Dify document)
// ---------------------------------------------------------------------------

const INPUT_PARSER_SYSTEM: &str = r#"# Role
你是 ResearchBot 的输入解析模块。

# Task
解析用户的调研请求，提取结构化参数。

# Output Format（严格 JSON）
{"product_category": "用户要调研的品类名称（英文标准化）","product_category_cn": "品类中文名","target_market": ["US"],"target_platform": "Amazon","analysis_depth": "standard","specific_questions": ["用户特别想了解的问题"],"competitor_asins": ["用户提供的竞品ASIN"],"budget_range": "用户提到的预算范围或null","is_new_seller": true}

# Rules
1. product_category 使用英文标准品类名（如 "Wireless Bluetooth Earbuds"）
2. 如果用户没有指定 market，默认为 ["US"]
3. 如果用户没有指定 platform，默认为 "Amazon"
4. analysis_depth:
   - "quick" = 用户只需要简单概览
   - "standard" = 默认深度
   - "deep" = 用户要求详细/深度分析
5. 如果用户输入过于模糊（如只说"帮我调研一下"没说品类），返回：
{"status": "need_more_info","follow_up": "请告诉我您想调研的产品品类是什么？例如：蓝牙耳机、瑜伽垫、手机壳等"}"#;

const MARKET_OVERVIEW_SYSTEM: &str = r#"# Role
你是一位资深的跨境电商市场分析师，拥有丰富的品类调研经验。

# Task
基于知识库数据和你的专业知识，为用户提供品类在目标市场的概况分析。

# 分析维度与输出

## 📊 市场概况分析报告

### 1. 市场规模与趋势
- 该品类在目标市场的整体规模（定性评估：大/中/小）
- 增长趋势（快速增长/稳步增长/成熟稳定/下滑）
- 季节性特征（哪些月份是旺季/淡季）
- 关键驱动因素（技术创新/消费升级/政策变化等）

### 2. 竞争格局
- 竞争强度评估：⭐⭐⭐⭐⭐（1-5星）
- 头部品牌/卖家（基于知识库案例数据）
- 中国卖家占比（定性评估：高/中/低）
- 新卖家进入难度评估
- 价格区间分布

### 3. 关键词分析
基于SEO关键词库中的数据：
- 核心搜索词及搜索量
- 搜索趋势变化（哪些词在上升/下降）
- 竞价水平（广告竞争度）
- 关键词机会（搜索量高但竞争度低的词）

### 4. 消费者画像
- 目标消费者特征（年龄/性别/收入水平）
- 核心购买动机
- 决策因素排序（价格/品质/品牌/功能/外观）
- 购买频率和复购率

### 5. 进入机会评估
- 综合推荐指数：⭐⭐⭐⭐⭐（1-5星）
- 机会点（市场空白/未被满足的需求）
- 风险点（合规风险/竞争风险/供应链风险）
- 建议切入角度

# Rules
1. 如果知识库中有该品类的真实数据，优先使用并标注"基于数据"
2. 如果需要用你的分析推断，标注"基于分析推断"
3. 数据不确定时给范围而非精确值
4. 所有价格使用目标市场的当地货币
5. 使用中文输出"#;

const COMPETITOR_ANALYSIS_SYSTEM: &str = r#"# Role
你是一位 Listing 逆向工程专家，擅长拆解爆款产品的 Listing 策略。

# Task
基于知识库中该品类的爆款案例数据，进行深度竞品 Listing 分析。

# 分析输出

## 🔍 竞品 Listing 深度分析

### 1. Top 产品 Listing 结构拆解

对知识库中收录的该品类 BSR Top 产品进行分析：

#### 标题策略分析
| 排名 | 品牌 | 标题结构 | 字符数 | 核心关键词 |
|------|------|---------|--------|-----------|
| #1 | XXX | [拆解结构] | XXX | [关键词列表] |
| #2 | XXX | [拆解结构] | XXX | [关键词列表] |

**标题共性总结：**
- 最常见的标题结构公式：...
- 出现频率最高的关键词 Top 10：...
- 标题平均长度：...

#### Bullet Points 策略分析
- 各卖家 Bullet 的通用结构模式
- 最常被强调的卖点 Top 5
- 数据/参数的展示方式
- 情感化表达 vs 技术化表达的比例

#### 定价策略分析
| 价格区间 | 产品数量 | BSR排名范围 | 评论数量范围 | Review评分 |
|---------|---------|------------|------------|-----------|
| $XX-XX | X个 | #X-#X | XXX-XXXX | X.X |

**定价洞察：**
- 最佳性价比区间：$XX - $XX
- 新品建议入场价格：$XX
- 价格敏感度评估

### 2. Review 洞察分析

基于Top产品的评论数据：

#### 消费者最爱的功能 Top 5
| 排名 | 功能/特点 | 提及频率 | 示例好评 |
|------|---------|---------|---------|
| 1 | [功能] | XX% | "..." |

#### 消费者最大的痛点 Top 5
| 排名 | 痛点 | 提及频率 | 示例差评 |
|------|------|---------|---------|
| 1 | [痛点] | XX% | "..." |

#### 竞品最薄弱的环节
[分析竞品集中收到差评的方面，这是你的差异化机会]

### 3. 视觉策略分析
- 主图风格趋势（白底/场景图/对比图）
- 图片数量平均值
- A+ Content 使用率
- 视频使用率

# Rules
1. 尽可能基于知识库中的真实案例数据
2. 如果案例数据不足，基于品类特征推断，但要标注
3. 分析要具有可操作性——不要只描述现状，要给出"所以你应该怎么做"
4. 使用中文输出"#;

const DIFFERENTIATION_ADVISOR_SYSTEM: &str = r#"# Role
你是一位跨境电商战略顾问，擅长帮助新卖家找到差异化切入点。

# Task
综合前面的分析，给出具体、可执行的差异化策略建议。

# Output

## 🎯 差异化策略建议

### 1. 产品差异化
基于竞品弱点和消费者未满足需求，建议：

**策略A（推荐）：** [具体差异化方向]
- 目标痛点：[瞄准哪个竞品薄弱环节]
- 具体做法：[产品层面如何差异化]
- 预估投入：[需要的额外成本]
- 风险评估：低/中/高

**策略B（备选）：** [另一个差异化方向]

### 2. Listing 差异化
**标题策略建议：**
- 建议标题结构：[具体公式]
- 建议主打关键词：[具体关键词列表]
- 与竞品标题的差异化点：[怎么做到不同]

**Bullet Points 策略建议：**
- 5条 Bullet 应该分别主打：
  1. [卖点及写法建议]
  2. [卖点及写法建议]
  3. [卖点及写法建议]
  4. [卖点及写法建议]
  5. [卖点及写法建议]

**图片/视觉建议：**
- 主图建议：...
- A+内容建议：...

### 3. 定价策略
- 建议首发价格：$XX
- 定价依据：[为什么这个价格]
- 促销策略：[上架初期的促销建议]
- 涨价路径：[何时/如何逐步提价]

### 4. 运营策略
- 上架时间建议：[考虑季节性]
- 初期推广建议：[广告策略概要]
- Review积累策略：[合规的获评方式]
- 库存建议：[首批进货量建议]

### 5. ⚠️ 风险提示
[列出需要注意的风险和规避方法]

### 6. 📅 30天行动计划
| 时间 | 行动项 | 预期成果 |
|------|--------|---------|
| Day 1-5 | ... | ... |
| Day 6-10 | ... | ... |
| Day 11-20 | ... | ... |
| Day 21-30 | ... | ... |

# Rules
1. 建议必须具体可执行，不要说"提高产品质量"这种废话
2. 考虑用户是否为新卖家，给出对应的建议级别
3. 资金有限的情况下，标注优先级（先做什么后做什么）
4. 使用中文输出"#;

// ---------------------------------------------------------------------------
// ResearchBot struct
// ---------------------------------------------------------------------------

pub(crate) struct ResearchBot;

impl ResearchBot {
    pub(crate) fn new() -> Self {
        Self
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

        // Step 1: Input parse
        {
            let start = Instant::now();
            let step = match step_input_parser(&mut ctx, llm).await {
                Ok(s) => s,
                Err(e) => {
                    return build_partial_response(
                        request_id,
                        params.agent_type,
                        steps,
                        format!("InputParser failed: {:?}", e),
                    );
                }
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Step 2: Market analysis
        {
            let start = Instant::now();
            let step = match step_market_overview(&mut ctx, llm).await {
                Ok(s) => s,
                Err(e) => {
                    return build_partial_response(
                        request_id,
                        params.agent_type,
                        steps,
                        format!("MarketOverview failed: {:?}", e),
                    );
                }
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Step 3: Competitor analysis
        {
            let start = Instant::now();
            let step = match step_competitor_analysis(&mut ctx, llm).await {
                Ok(s) => s,
                Err(e) => {
                    return build_partial_response(
                        request_id,
                        params.agent_type,
                        steps,
                        format!("CompetitorAnalysis failed: {:?}", e),
                    );
                }
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Step 4: Strategy generation
        {
            let start = Instant::now();
            let step = match step_differentiation_advisor(&mut ctx, llm).await {
                Ok(s) => s,
                Err(e) => {
                    return build_partial_response(
                        request_id,
                        params.agent_type,
                        steps,
                        format!("DifferentiationAdvisor failed: {:?}", e),
                    );
                }
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Step 5: Report assemble (non-LLM)
        {
            let start = Instant::now();
            let step = step_report_assembler(&mut ctx);
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
                "agent": "ResearchBot",
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

async fn step_input_parser(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let user_input = ctx_str(ctx, "user_input");

    let user_message = format!("用户请求: {}", user_input);
    let response = llm
        .chat("gpt-4o-mini", INPUT_PARSER_SYSTEM, &user_message)
        .await?;

    let parsed: Value = serde_json::from_str(&response).unwrap_or_else(|_| {
        json!({
            "product_category": "General",
            "product_category_cn": "通用品类",
            "target_market": ["US"],
            "target_platform": "Amazon",
            "analysis_depth": "standard",
            "specific_questions": [],
            "competitor_asins": [],
            "budget_range": null,
            "is_new_seller": true
        })
    });

    ctx.insert("input_parser".to_string(), parsed.clone());

    Ok(StepOutput {
        step_name: "input_parser".to_string(),
        model: "gpt-4o-mini".to_string(),
        output: parsed,
    })
}

async fn step_market_overview(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let parsed_input = ctx
        .get("input_parser")
        .cloned()
        .unwrap_or(json!({}));

    let product_category = parsed_input
        .get("product_category")
        .and_then(|v| v.as_str())
        .unwrap_or("General");
    let target_market = parsed_input
        .get("target_market")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "US".to_string());
    let target_platform = parsed_input
        .get("target_platform")
        .and_then(|v| v.as_str())
        .unwrap_or("Amazon");

    let user_message = format!(
        "品类: {}\n目标市场: {}\n平台: {}\n\n请提供该品类的市场概况分析。",
        product_category, target_market, target_platform,
    );

    let response = llm
        .chat("claude-3-5-sonnet-20241022", MARKET_OVERVIEW_SYSTEM, &user_message)
        .await?;

    let output = json!({
        "market_report": response,
    });

    ctx.insert("market_overview".to_string(), json!(response));

    Ok(StepOutput {
        step_name: "market_overview".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        output,
    })
}

async fn step_competitor_analysis(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let parsed_input = ctx
        .get("input_parser")
        .cloned()
        .unwrap_or(json!({}));

    let product_category = parsed_input
        .get("product_category")
        .and_then(|v| v.as_str())
        .unwrap_or("General");

    let user_message = format!(
        "品类: {}\n\n请对该品类的竞品 Listing 进行深度分析。",
        product_category,
    );

    let response = llm
        .chat("claude-3-5-sonnet-20241022", COMPETITOR_ANALYSIS_SYSTEM, &user_message)
        .await?;

    let output = json!({
        "competitor_report": response,
    });

    ctx.insert("competitor_analysis".to_string(), json!(response));

    Ok(StepOutput {
        step_name: "competitor_analysis".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        output,
    })
}

async fn step_differentiation_advisor(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let market_overview = ctx_str(ctx, "market_overview");
    let competitor_analysis = ctx_str(ctx, "competitor_analysis");
    let parsed_input = ctx
        .get("input_parser")
        .cloned()
        .unwrap_or(json!({}));

    let is_new_seller = parsed_input
        .get("is_new_seller")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);
    let budget_range = parsed_input
        .get("budget_range")
        .and_then(|v| v.as_str())
        .unwrap_or("未指定");
    let specific_questions: String = parsed_input
        .get("specific_questions")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join("; ")
        })
        .unwrap_or_else(|| "无".to_string());

    let user_message = format!(
        "市场概况分析:\n{market}\n\n竞品分析:\n{competitor}\n\n用户背景: 新卖家={is_new}, 预算={budget}\n特别关注的问题: {questions}",
        market = market_overview.chars().take(1500).collect::<String>(),
        competitor = competitor_analysis.chars().take(1500).collect::<String>(),
        is_new = is_new_seller,
        budget = budget_range,
        questions = specific_questions,
    );

    let response = llm
        .chat("claude-3-5-sonnet-20241022", DIFFERENTIATION_ADVISOR_SYSTEM, &user_message)
        .await?;

    let output = json!({
        "strategy_report": response,
    });

    ctx.insert("differentiation_advisor".to_string(), json!(response));

    Ok(StepOutput {
        step_name: "differentiation_advisor".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        output,
    })
}

fn step_report_assembler(ctx: &mut WorkflowContext) -> StepOutput {
    let parsed_input = ctx
        .get("input_parser")
        .cloned()
        .unwrap_or(json!({}));
    let product_category_cn = parsed_input
        .get("product_category_cn")
        .and_then(|v| v.as_str())
        .unwrap_or("品类名");
    let product_category = parsed_input
        .get("product_category")
        .and_then(|v| v.as_str())
        .unwrap_or("Product Category");
    let target_market = parsed_input
        .get("target_market")
        .and_then(|v| v.as_array())
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        })
        .unwrap_or_else(|| "US".to_string());
    let target_platform = parsed_input
        .get("target_platform")
        .and_then(|v| v.as_str())
        .unwrap_or("Amazon");

    let market_overview = ctx_str(ctx, "market_overview");
    let competitor_analysis = ctx_str(ctx, "competitor_analysis");
    let differentiation_advisor = ctx_str(ctx, "differentiation_advisor");

    let markdown = format!(
        r#"# 📊 {product_category_cn} — 跨境电商市场调研报告

> **调研品类：** {product_category}
> **目标市场：** {target_market}
> **目标平台：** {target_platform}
> **报告来源：** ResearchBot

---

## 第一部分：市场概况

{market_overview}

---

## 第二部分：竞品分析

{competitor_analysis}

---

## 第三部分：差异化策略与行动建议

{differentiation_advisor}

---

## 免责声明

> 本报告基于AI分析和已有数据生成，仅供参考。市场情况实时变化，建议结合最新数据做出决策。报告中标注"基于分析推断"的部分为AI推理结论，请谨慎采用。

---

💡 **需要下一步帮助？**
- 使用 **ListingBot Pro** 直接生成该品类的产品 Listing
- 使用 **AdsBot** 生成该品类的广告文案
"#,
        product_category_cn = product_category_cn,
        product_category = product_category,
        target_market = target_market,
        target_platform = target_platform,
        market_overview = market_overview,
        competitor_analysis = competitor_analysis,
        differentiation_advisor = differentiation_advisor,
    );

    ctx.insert("final_markdown".to_string(), json!(markdown));

    StepOutput {
        step_name: "report_assembler".to_string(),
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
