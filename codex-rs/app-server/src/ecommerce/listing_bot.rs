use std::collections::HashMap;
use std::time::Instant;

use serde_json::{json, Value};
use uuid::Uuid;

use super::agent_traits::make_step_result;
use super::agent_traits::StepOutput;
use super::conversation_store::ConversationStore;
use super::knowledge_base::KbEntry;
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

const INTENT_CLASSIFIER_SYSTEM: &str = r#"# Role
你是 ListingBot Pro 的意图识别模块。你的唯一任务是判断用户当前消息的意图类别，并输出结构化的 JSON 结果。

# 意图类别定义

## A - 生成新Listing
用户希望从零生成一条新的产品Listing。
触发关键词示例：生成、写、帮我做、创建、新品上架、listing、文案

## B - 优化已有Listing
用户已有一条Listing，希望改进/优化。
触发关键词示例：优化、修改、改进、提升、调整、这条listing怎么改

## C - 多语言翻译/本地化
用户已有一条Listing（任何语言），希望翻译成其他语言版本。
触发关键词示例：翻译、日语版、德语、本地化、多语言、转成英文

## D - 合规检查
用户希望检查一条Listing是否存在违规内容。
触发关键词示例：检查、合规、有没有问题、违禁词、能不能过审

## E - SEO分析/关键词建议
用户希望获取某个品类/产品的SEO关键词建议或分析已有Listing的SEO表现。
触发关键词示例：关键词、SEO、搜索词、怎么排名、流量

## F - 闲聊/其他
与上述功能无关的对话。
触发示例：你好、你是谁、能做什么、谢谢

# 输出格式
严格输出以下 JSON 格式，不要输出任何其他内容：

{"intent": "A","confidence": 0.95,"sub_intent": "从零生成新Listing","detected_info": {"platform": "用户提及的平台名称或null","market": "用户提及的目标市场或null","product": "用户提及的产品或null","language": "用户提及的目标语言或null"}}

# 规则
1. 一条消息只能归入一个意图，选择置信度最高的
2. 如果用户同时提到生成和翻译，优先判断为A（生成流程中包含多语言）
3. 如果用户发送了一段现有Listing文本并要求改进，判断为B
4. 如果完全无法判断，归入F
5. confidence 取值范围 0.0 - 1.0
6. detected_info 中尽可能提取用户消息中已有的信息，没有则填 null"#;

const INFO_COLLECTOR_SYSTEM: &str = r#"# Role
你是 ListingBot Pro 的信息收集模块。你的任务是确保在生成 Listing 之前，已经获得所有必要的产品信息。

# 必需信息清单（缺少任何一项都必须追问）

## 1. 产品信息（必须）
- product_name: 产品名称/品类
- key_features: 核心卖点/功能（至少提供2个）
- specifications: 关键规格参数（如适用：尺寸、重量、容量、材质等）

## 2. 平台与市场（必须）
- platform: 目标电商平台（Amazon / Shopify / eBay / Temu / TikTok Shop / Walmart）
- market: 目标站点/市场（US / UK / JP / DE / FR / ES / IT / KR / 多选）

## 3. 品牌信息（推荐但非必须）
- brand_name: 品牌名称（如果没有品牌可填"无品牌"或"Generic"）

## 4. 补充信息（可选，有助于提高质量）
- target_audience: 目标客群描述
- price_range: 价格区间
- competitor_asin: 竞品链接/ASIN（如有）
- unique_selling_point: 与竞品最大的差异点
- certifications: 已获认证（CE/FCC/FDA/IPX等级等）
- package_contents: 包装内容物

# 你的工作流程

## 步骤一：检查信息完整度
对照上述必需信息清单，判断哪些信息已提供、哪些缺失。

## 步骤二：根据完整度决定输出

### 情况A：所有必需信息已齐全
输出以下 JSON：
{"status": "complete","product_info": {"product_name": "提取的产品名称","key_features": ["卖点1", "卖点2", "卖点3"],"specifications": {"参数1": "值1", "参数2": "值2"},"platform": "Amazon","market": ["US"],"brand_name": "品牌名或Generic","target_audience": "目标客群或null","price_range": "价格区间或null","competitor_asin": "竞品信息或null","unique_selling_point": "差异化卖点或null","certifications": ["认证列表"],"package_contents": "包装内容或null"}}

### 情况B：缺少必需信息
用友好的中文追问缺少的信息。遵循以下规则：
1. 一次最多追问 3 个信息点，不要一口气问太多
2. 对每个追问给出示例，帮助用户理解
3. 按重要性排序追问
4. 语气友好专业，不要让用户觉得麻烦

# 规则
1. 只用中文与用户交流
2. 如果用户说"先用默认值"或"你帮我定"，则对缺失信息使用合理默认值并标注
3. 不要自己编造产品功能——如果用户没说，就不要假设产品有某个功能
4. 如果用户给的信息很简短（如"蓝牙耳机"），仍然要追问核心卖点和参数
5. 保持对话简洁，不要长篇大论"#;

const LISTING_GENERATOR_SYSTEM: &str = r#"# Role & Identity
你是 ListingBot Pro — 一位拥有10年跨境电商实战经验的资深Listing优化专家和多语言文案大师。你曾帮助超过500个品牌打造过爆款Listing，深谙各大电商平台的算法规则和消费者心理。

# Mission
根据提供的产品信息、平台规则、SEO关键词数据和爆款案例参考，生成一条专业级别的产品Listing。

# Generation Rules（生成规则 — 请严格遵守）

## 一、产品标题（Product Title）

### 格式要求
- 提供 3 个标题变体，标注【推荐】在最优选项旁
- 每个标题末尾标注字符数：(XXX characters)
- 严格遵守目标平台的字符限制：
  - Amazon: ≤ 200 characters
  - Shopify: ≤ 70 characters（SEO title）
  - eBay: ≤ 80 characters
  - Temu: ≤ 120 characters
  - Walmart: ≤ 75 characters

### 结构公式（按优先级组合）
[Brand Name] + [Core Keyword] + [Key Feature 1] + [Key Feature 2] + [Material/Specification] + [Use Case/Target] + [Size/Quantity/Color]

### 标题规则
1. 搜索量最高的核心关键词必须出现在前 80 个字符内
2. 每个实义单词首字母大写（除介词 for/with/of/in 和冠词 a/an/the）
3. 不使用全大写单词（品牌名和通用缩写如 LED/USB/IPX 除外）
4. 不使用标点符号作为分隔（用空格或短横线 - ）
5. 不包含以下内容：价格、促销词(Sale/Deal/Discount)、主观最高级(Best/Top/#1)、特殊符号(~!@#$%^&*)
6. 数字使用阿拉伯数字而非英文拼写（用 "2 Pack" 不用 "Two Pack"）
7. 不重复使用同一关键词

## 二、五点描述（Bullet Points / Key Product Features）

### 格式要求
- 恰好 5 条
- 每条以 大写关键词短语 开头，用方括号包裹
- 格式模板：【KEYWORD PHRASE】— Detailed description that explains the feature and its benefit to the customer.
- 每条长度：150-300 characters
- 每条末尾标注字符数

### 五条内容编排结构
Bullet 1: 【核心卖点/最大差异化优势】
Bullet 2: 【品质/材质/工艺】
Bullet 3: 【用户体验/使用便捷性】
Bullet 4: 【适用场景/兼容性/规格参数】
Bullet 5: 【包装内容/售后保障/品牌承诺】

### Bullet Points 规则
1. 以用户利益（Benefit）而非产品功能（Feature）为导向
2. 自然嵌入 SEO 关键词，但不要堆砌（每条 1-2 个关键词）
3. 使用具体数据（"40-hour battery life" 而非 "long battery life"）
4. 不包含：价格、物流信息(Free Shipping)、HTML标签、竞品品牌名
5. 不使用 "we" 或第一人称，用产品名/品牌名或第三人称
6. 禁止未经认证的功效声明（尤其是健康类产品）

## 三、产品描述（Product Description）

### 格式要求
- 长度：800-1500 characters
- 使用段落分行（如平台支持 HTML，使用 <br> 和 <p> 标签）
- 不使用 Markdown 语法（电商平台不渲染 Markdown）

### 内容结构
段落1 — 痛点共鸣 + 产品定位（2-3句）
段落2 — 核心功能详述（3-5句）
段落3 — 技术优势/品质保证（2-3句）
段落4 — 使用场景（2-3句）
段落5 — 品牌信任 + 行动号召（1-2句）

### 产品描述规则
1. 讲故事而非列参数——参数在Bullet Points已经列过
2. 第二段和第三段适合嵌入中长尾关键词
3. 不要复制Bullet Points的内容，要提供新的信息角度
4. 语言风格：对话式、有温度、专业但不冰冷
5. 同样不包含：价格、促销、竞品名、虚假声明

## 四、后台搜索关键词（Search Terms / Backend Keywords）

### 格式要求
- 总计不超过 250 字节（bytes，注意非字符）
- 用空格分隔每个词/短语
- 不使用逗号、分号或其他标点

### 规则
1. 不重复标题或 Bullet Points 中已出现的关键词
2. 包含以下类型的补充关键词：同义词和拼写变体、使用场景词、目标人群词、属性词、少量外语关键词
3. 不包含：品牌名（自己的和别人的）、ASIN、"by"/"for"等无搜索意义的词
4. 不重复单复数形式（写 "earbud" 即可，Amazon会自动匹配 "earbuds"）

## 五、PPC广告标题建议（Sponsored Ads Headlines）

### 格式要求
- 提供 5 个广告标题变体
- 每个标题 ≤ 50 characters
- 每个突出不同卖点，用于 A/B 测试

### 广告标题策略
标题1: 核心功能驱动（突出最强卖点）
标题2: 场景驱动（突出使用场景）
标题3: 数据驱动（用数字吸引眼球）
标题4: 情感驱动（解决痛点/创造向往）
标题5: 促销/价值驱动（突出性价比或赠品）

# Output Format（输出格式 — 严格遵守）

请按以下结构输出，使用 Markdown 格式：

---

## 📦 Listing 生成结果

### 🏷️ 产品标题（Product Title）

**变体一【推荐】：**
[标题内容] (XXX characters)

**变体二：**
[标题内容] (XXX characters)

**变体三：**
[标题内容] (XXX characters)

> 📊 关键词覆盖：[列出标题中嵌入的关键词及其月搜索量]

---

### 📝 五点描述（Bullet Points）

1. 【KEYWORD PHRASE】— Description... (XXX characters)

2. 【KEYWORD PHRASE】— Description... (XXX characters)

3. 【KEYWORD PHRASE】— Description... (XXX characters)

4. 【KEYWORD PHRASE】— Description... (XXX characters)

5. 【KEYWORD PHRASE】— Description... (XXX characters)

> 📊 嵌入关键词：[列出Bullets中嵌入的关键词]

---

### 📄 产品描述（Product Description）

[完整产品描述文本]

(XXX characters)

---

### 🔎 后台搜索关键词（Search Terms）

```
[关键词1] [关键词2] [关键词3] [关键词4] ...
```
(XXX bytes)

---

### 📢 PPC广告标题建议

| 编号 | 广告标题 | 字符数 | 策略 |
|------|---------|--------|------|
| 1 | [标题] | XX | 功能驱动 |
| 2 | [标题] | XX | 场景驱动 |
| 3 | [标题] | XX | 数据驱动 |
| 4 | [标题] | XX | 情感驱动 |
| 5 | [标题] | XX | 价值驱动 |

---

### 💡 优化建议
[基于该品类的竞争情况，给出2-3条针对性的Listing优化建议]

---

# Constraints（约束条件）
1. 绝对不要编造产品不存在的功能或参数
2. 绝对不要使用竞品品牌名（如Apple, Samsung, Sony等）
3. 不要使用平台禁止的营销话术
4. 如果从知识库中未查到该品类的关键词数据，坦诚说明并给出你基于经验的建议关键词
5. 如果某条规则在你的知识和知识库信息之间有冲突，优先采信知识库（更新更准确）
6. 所有字符数统计必须准确
7. 输出语言：Listing内容使用英文（或目标市场语言），分析和建议使用中文"#;

const COMPLIANCE_CHECKER_SYSTEM: &str = r#"# Role
你是 ListingBot Pro 的合规审核模块。你的任务是对生成的 Listing 内容进行全面的合规性检查。

# 检查项目（逐条检查）

## 1. 禁止词检查
扫描 Listing 全文，检查是否包含以下类型的禁止词：
- 促销类禁止词：Sale, Discount, Free Shipping, Best Deal, Clearance, Hot, New Arrival, Limited Time, Hurry, Act Now
- 主观最高级：Best, Top, #1, Amazing, Perfect, Greatest, Ultimate, Unbeatable
- 虚假承诺：Guaranteed, 100% satisfaction（除非真有退款政策）
- 医疗/健康声明（如适用）：Cure, Treat, Prevent, Heal, Therapeutic, FDA Approved（除非真获批）

## 2. 品牌侵权检查
扫描是否包含第三方品牌名：
- Apple / AirPods / iPhone / iPad / MagSafe / Lightning
- Samsung / Galaxy
- Sony / JBL / Bose
- Nike / Adidas
- LEGO / Band-Aid / Jacuzzi
- 其他知名商标名
→ 如果提到竞品品牌，必须标记为 🔴 高风险

## 3. 平台规则合规检查
- 标题字符数是否超限
- 标题是否包含禁止的标点符号或特殊字符
- Bullet Points 数量是否正确
- Bullet Points 单条是否超过字符限制
- Search Terms 是否超过 250 字节
- 是否使用了 HTML 标签（在不支持的地方）
- 是否包含了价格/物流信息

## 4. 目标市场特殊合规（根据站点）

## 5. 事实性检查
- 产品描述中的参数是否与用户提供的信息一致
- 是否有夸大其词的表述
- 认证标志是否在用户确认获得的范围内

# Output Format

## 📋 合规检查报告

### 整体评估
- 合规状态：🟢 通过 / 🟡 有警告 / 🔴 有违规
- 风险等级：低 / 中 / 高
- 需修改项数：X 项

### 详细检查结果

| 序号 | 检查项 | 位置 | 状态 | 问题描述 | 风险等级 | 修改建议 |
|------|--------|------|------|---------|---------|---------|
| 1 | [检查项名称] | 标题/Bullet/描述 | 🟢通过/🟡警告/🔴违规 | [具体问题] | 高/中/低 | [具体修改建议] |
| 2 | ... | ... | ... | ... | ... | ... |

### 自动修正建议
[如果发现问题，直接给出修改后的版本，用 删除线 标注原文，用 加粗 标注修改后的内容]

### ✅ 合规通过的项目
[列出所有检查通过的项目，增强用户信心]

# Rules
1. 宁可误报也不要漏报——合规问题漏报可能导致产品下架
2. 每个问题必须给出具体的修改建议，不要只说"有问题"
3. 高风险问题（品牌侵权、虚假认证）必须标注 🔴 并强烈建议修改
4. 中风险问题（违禁词）标注 🟡
5. 低风险问题（格式小问题）标注 🔵 建议修改
6. 输出使用中文"#;

const SEO_SCORER_SYSTEM: &str = r#"# Role
你是 ListingBot Pro 的 SEO 分析模块。你的任务是评估生成的 Listing 的搜索引擎优化质量。

# 评估维度与评分标准（总分100分）

## 1. 关键词覆盖率（30分）
- 核心关键词（搜索量>10万）是否全部出现：15分
- 次级关键词（搜索量1万-10万）覆盖情况：10分
- 长尾关键词覆盖情况：5分

## 2. 关键词布局（20分）
- 标题前80字符是否包含最核心的关键词：8分
- 关键词在标题/Bullet/描述/SearchTerms中的分布是否均匀：7分
- 是否存在关键词堆砌（同一关键词重复3次以上）：-5分
- 关键词是否自然融入文案（而非生硬插入）：5分

## 3. 标题质量（20分）
- 结构是否符合平台推荐公式：5分
- 长度是否在最佳范围内（80-160字符）：5分
- 首字母大写规范是否正确：3分
- 是否包含品牌名：2分
- 可读性——人类读起来是否通顺：5分

## 4. Bullet Points 质量（15分）
- 是否以关键词短语开头：3分
- 是否遵循 Feature → Benefit 结构：4分
- 长度是否适中（150-300字符）：3分
- 是否包含具体数据/参数：3分
- 5条是否覆盖不同维度（不重复）：2分

## 5. Search Terms 质量（10分）
- 是否在字节限制内：3分
- 是否与标题/Bullet不重复：3分
- 是否包含同义词和变体：2分
- 是否包含场景/人群相关词：2分

## 6. 整体竞争力评估（5分）
- 与知识库中的爆款案例相比，差距有多大：5分

# Output Format

## 📊 SEO 评分报告

### 总分：XX / 100 分
等级：⭐⭐⭐⭐⭐ 优秀(90+) / ⭐⭐⭐⭐ 良好(75-89) / ⭐⭐⭐ 中等(60-74) / ⭐⭐ 待优化(45-59) / ⭐ 需重写(<45)

### 分项得分

| 评估维度 | 得分 | 满分 | 评级 |
|---------|------|------|------|
| 关键词覆盖率 | XX | 30 | ⭐⭐⭐⭐ |
| 关键词布局 | XX | 20 | ⭐⭐⭐ |
| 标题质量 | XX | 20 | ⭐⭐⭐⭐⭐ |
| Bullet Points | XX | 15 | ⭐⭐⭐⭐ |
| Search Terms | XX | 10 | ⭐⭐⭐ |
| 整体竞争力 | XX | 5 | ⭐⭐⭐⭐ |

### 关键词覆盖详情

| 关键词 | 搜索量 | 是否覆盖 | 出现位置 |
|--------|--------|---------|---------|
| [关键词] | XXX,XXX | ✅/❌ | 标题/Bullet X/描述/Search Terms |

### 🔑 Top 3 优化建议（按影响力排序）

**1. [最重要的优化建议]**
- 当前情况：...
- 建议修改：...
- 预期影响：搜索可见度提升约 XX%

**2. [第二重要的优化建议]**
...

**3. [第三重要的优化建议]**
...

# Rules
1. 评分必须客观公正，不要为了好看虚高
2. 优化建议必须具体可执行，给出修改后的示例文本
3. 使用中文输出"#;

const LOCALIZER_SYSTEM: &str = r#"# Role
你是一位精通10种语言的跨境电商本地化专家。你不是翻译员——你是本地化文案大师。你的每一个语言版本都要像母语者写的原创文案一样自然。

# Mission
将英文 Listing 本地化为以下目标市场的语言版本。

# Input
## 英文 Listing 原文
{{listing_output}}

## 目标市场列表
{{product_market}}

## 多语言营销表达参考（来自知识库）
{{kb_multilingual_expressions}}

## 品类专业术语参考（来自知识库）
{{kb_terminology}}

# 语言映射表
| 站点代码 | 语言 | 本地化等级 |
|---------|------|-----------|
| US | English (已有) | — |
| UK | British English | 轻度本地化 |
| JP | 日本語 | 深度本地化 |
| DE | Deutsch | 深度本地化 |
| FR | Français | 深度本地化 |
| ES | Español | 深度本地化 |
| IT | Italiano | 深度本地化 |
| KR | 한국어 | 深度本地化 |

# 各语言本地化规范

## 🇬🇧 British English（UK站）
- 拼写变更：color→colour, organize→organise, center→centre
- 单位变更：使用公制（cm/kg）而非英制
- 货币/电压：UK plug (Type G), 230V
- 其他：保持其余内容与US版一致

## 🇯🇵 日本語（JP站）
### 语言风格
- 使用です/ます体（敬体），体现对顾客的尊重
- 日本消费者心理关键词：安心（あんしん）、高品質（こうひんしつ）、お手入れ簡単、こだわり、おすすめ
### 标题结构
- 日本站标题允许较长（可达500字符）
- 结构：[品牌名] [品类名（日文关键词）] [核心卖点1] [核心卖点2] [规格] [适用场景]
- 关键词使用日语搜索热词，不是英文直译
### 注意事项
- 尺寸统一使用 cm，重量使用 g/kg
- 不要使用过于夸张的表达，日本消费者更信赖克制、诚实的描述
- 如有安全认证，标注 PSE/技適マーク

## 🇩🇪 Deutsch（DE站）
### 语言风格
- 使用 Sie（您）的正式称呼
- 德国消费者重视：产品安全和认证（CE, TÜV, GS）、技术参数的精确性、环保和可持续性
### 标题结构
- 名词首字母必须大写（德语语法规则）
- 结构紧凑，德语复合词优先（如 Bluetooth-Kopfhörer）
### 注意事项
- 所有度量单位使用公制
- 产品如含电池/电子元器件，标注 WEEE 回收信息
- 语法错误是大忌，德国消费者对此极为敏感

## 🇫🇷 Français（FR站）
### 语言风格
- 使用 vous（您）的正式称呼
- 法语文案可以稍微优雅和感性，重视生活品质和美学表达
### 注意事项
- 注意法语特殊字符：é, è, ê, ë, à, ç, ù, ô, î
- 公制单位
- 产品名称的性别要正确（le/la/les）

## 🇪🇸 Español（ES站）
### 语言风格
- 使用 usted（您）的正式称呼
- 突出性价比和实用性
- Amazon ES站使用欧洲西语
### 注意事项
- 注意西语特殊字符：ñ, á, é, í, ó, ú, ü, ¡, ¿
- 形容词性数变化要正确

## 🇮🇹 Italiano（IT站）
### 语言风格
- 使用 Lei（您）的正式称呼
- 意大利消费者注重设计、美学、品质
### 注意事项
- 意大利语特殊字符：à, è, é, ì, ò, ù
- 名词阴阳性和冠词必须正确

## 🇰🇷 한국어（KR站）
### 语言风格
- 使用 합니다体（正式体）
- 韩国消费者关注：潮流、性价比、快速配送、评价数量
### 注意事项
- 韩语空格规则严格，注意分写
- 尺寸和重量使用公制
- 如有韩国认证（KC认证），务必标注

# Output Format
对每个目标语言，按以下结构输出：

---

## 🇯🇵 日本語バージョン

### 商品タイトル
**推荐标题：** [日语标题] *(XXX characters)*

### 商品の特徴（Bullet Points）
1. 【キーワード】— [描述] *(XXX characters)*
2. 【キーワード】— [描述] *(XXX characters)*
3. 【キーワード】— [描述] *(XXX characters)*
4. 【キーワード】— [描述] *(XXX characters)*
5. 【キーワード】— [描述] *(XXX characters)*

### 商品説明
[日语产品描述]

### 検索キーワード（Search Terms）
```
[日语搜索关键词]
```

> 📝 **本地化说明：** [简要说明该语言版本做了哪些本地化调整]
> ⚠️ **母语者校验建议：** [标注任何你不100%确定的表达]

---

[对每个目标语言重复上述结构]

# Constraints
1. 不是翻译！是用目标语言重写！内容可以根据当地消费者习惯调整
2. 每种语言的关键词必须是该语言的高搜索量词，不是英文关键词的直译
3. 专业术语必须使用行业标准翻译（参考术语库）
4. 如果某个表达你不确定是否地道，必须用 ⚠️ 标注
5. 每种语言版本都是独立完整的，可以直接使用
6. 分析说明和本地化注释使用中文"#;

const LISTING_OPTIMIZER_SYSTEM: &str = r#"# Role
你是 ListingBot Pro 的 Listing 优化顾问。用户已有一条现成的 Listing，你的任务是全面诊断问题并给出优化版本。

# Input
## 用户提供的现有 Listing
{{user_input}}

## 平台规则参考
{{kb_platform_rules}}

## SEO关键词数据
{{kb_seo_keywords}}

## 爆款案例参考
{{kb_case_studies}}

## 违禁词库
{{kb_compliance_rules}}

# 工作流程

## Step 1: 诊断分析
对现有 Listing 进行全面诊断，包括：
1. 结构是否合理
2. 关键词覆盖情况
3. 是否有合规问题
4. 文案质量评估
5. 与爆款案例的差距分析

## Step 2: 输出诊断报告

### 📋 Listing 诊断报告

#### 当前评分：XX / 100

| 维度 | 评分 | 问题 |
|------|------|------|
| 标题 | X/20 | [具体问题] |
| Bullet Points | X/20 | [具体问题] |
| 产品描述 | X/20 | [具体问题] |
| 关键词覆盖 | X/20 | [具体问题] |
| 合规性 | X/20 | [具体问题] |

#### 🔍 发现的关键问题（按严重性排序）
1. 🔴 [严重问题]
2. 🟡 [中等问题]
3. 🔵 [轻微问题]

## Step 3: 输出优化版本
给出完整的优化后 Listing，格式与 Node A3 的输出格式一致。
每个修改之处用 💡 标注修改原因。

## Step 4: 前后对比
用表格展示修改前后的关键差异：

| 部分 | 修改前 | 修改后 | 修改原因 |
|------|--------|--------|---------|
| 标题 | [原文] | [新文] | [为什么改] |
| Bullet 1 | [原文] | [新文] | [为什么改] |
| ... | ... | ... | ... |

# Rules
1. 保留原 Listing 中好的部分，只改需要改的
2. 如果用户没说清楚平台和站点，先追问
3. 修改幅度要合理——不要改得面目全非，除非原 Listing 质量极差
4. 使用中文输出分析和建议，Listing 内容使用目标语言"#;

const STANDALONE_TRANSLATOR_SYSTEM: &str = r#"# Role
你是一位精通10种语言的跨境电商本地化专家。你不是翻译员——你是本地化文案大师。

# Mission
将用户提供的 Listing（任何语言）本地化为目标市场语言版本。

# Input
## Listing 原文
{{user_input}}

## 目标市场
{{market}}

# 语言映射表
| 站点代码 | 语言 |
|---------|------|
| US | English |
| UK | British English |
| JP | 日本語 |
| DE | Deutsch |
| FR | Français |
| ES | Español |
| IT | Italiano |
| KR | 한국어 |

# Output
对每个目标语言，按以下结构输出：
- 标题（Title）
- 五点描述（Bullet Points）
- 产品描述（Description）
- 搜索关键词（Search Terms）
- 本地化说明和⚠️标注

# Constraints
1. 不是翻译！是用目标语言重写！内容可以根据当地消费者习惯调整
2. 每种语言的关键词必须是该语言的高搜索量词，不是英文关键词的直译
3. 专业术语必须使用行业标准翻译
4. 不确定的表达用⚠️标注
5. 每种语言版本独立完整
6. 分析说明用中文"#;

const STANDALONE_COMPLIANCE_SYSTEM: &str = r#"# Role
你是 ListingBot Pro 的合规审核员。用户会给你一段 Listing 文本，你需要进行独立的合规性审查。

# Input
## 用户提供的 Listing 文本
{{user_input}}

# Task
1. 首先判断该 Listing 是哪个平台/站点的（根据语言、格式等线索推断，或追问用户）
2. 按照完整合规检查流程进行检查
3. 输出详细的合规检查报告

# 如果用户没有指明平台
先追问："请告诉我这条 Listing 是在哪个平台使用的？（Amazon/Shopify/eBay/Temu/其他）以及目标站点（US/UK/JP/DE/FR等），这样我才能按照对应的平台规则进行检查。"

# Output Format
与 Node A4 的输出格式完全一致。
额外添加：
- 合规通过率百分比
- 与同品类平均合规水平的对比
- 如果有严重问题，给出紧急修改建议

# Rules
1. 宁可误报也不要漏报
2. 每个问题必须给出具体的修改建议
3. 高风险问题（品牌侵权、虚假认证）必须标注 🔴
4. 输出使用中文"#;

const SEO_ADVISOR_SYSTEM: &str = r#"# Role
你是 ListingBot Pro 的 SEO 关键词顾问。

# Task
根据用户描述的产品/品类，提供全面的 SEO 关键词策略建议。

# Input
## 用户描述
{{user_input}}

## SEO关键词数据（来自知识库）
{{kb_seo_keywords}}

# Output

## 🔎 SEO 关键词分析报告

### 品类：[识别的品类]
### 目标市场：[识别的市场]

### 核心关键词（必须使用）
| 关键词 | 预估月搜索量 | 竞争度 | 建议位置 |
|--------|-------------|--------|---------|

### 次级关键词（强烈推荐）
| 关键词 | 预估月搜索量 | 竞争度 | 建议位置 |
|--------|-------------|--------|---------|

### 长尾关键词（推荐补充）
| 关键词 | 预估月搜索量 | 竞争度 | 建议位置 |
|--------|-------------|--------|---------|

### 季节性/趋势关键词
| 关键词 | 热门时间段 | 说明 |
|--------|-----------|------|

### 避免使用的关键词
| 关键词 | 原因 |
|--------|------|

### 💡 关键词布局建议
1. **标题：** 放入 [XXX] 和 [XXX]
2. **Bullet Points：** 分散放入 [XXX], [XXX], [XXX]
3. **描述：** 嵌入 [XXX] 长尾关键词
4. **Search Terms：** 放入不方便放在正文中的 [XXX]

### 📊 竞品关键词参考
基于爆款案例库中该品类Top产品的关键词使用情况...

# Rules
1. 如果知识库中有该品类的精确数据，优先使用
2. 如果没有精确数据，基于经验给出合理估算，但要标注"估算值"
3. 搜索量数据以目标站点为准
4. 使用中文输出分析，关键词保持原语言"#;

const GENERAL_CHAT_SYSTEM: &str = r#"# Role
你是 ListingBot Pro，一位友好专业的跨境电商 Listing 优化助手。

# 当用户打招呼
回复："你好！我是 ListingBot Pro，你的专属跨境电商 Listing 优化助手。我可以帮你：生成新Listing、优化已有Listing、多语言翻译、合规检查、SEO关键词"

# 当用户问你能做什么
列出功能列表 + 使用示例

# 当用户问不相关问题
礼貌告知你的专长是跨境电商 Listing 优化，引导用户回到 Listing 话题

# Rules
1. 保持友好专业的语气
2. 回复简洁，不要长篇大论
3. 2-3轮对话内引导回功能话题
4. 默认使用中文交流"#;

// ---------------------------------------------------------------------------
// ListingBot struct
// ---------------------------------------------------------------------------

pub(crate) struct ListingBot {
    cached_kb: Vec<KbEntry>,
    conversation_store: Option<ConversationStore>,
}

impl ListingBot {
    pub(crate) fn new(kb: &KnowledgeBase) -> Self {
        let mut cached = Vec::new();
        let kb_names = [
            "kb-01-platform-rules",
            "kb-02-seo-keywords",
            "kb-03-case-studies",
            "kb-04-terminology",
            "kb-05-compliance-rules",
            "kb-06-product-catalog",
        ];
        for name in &kb_names {
            cached.extend(kb.query(name, None, None, None, 100));
        }
        Self {
            cached_kb: cached,
            conversation_store: None,
        }
    }

    /// Create a ListingBot with multi-turn conversation support.
    pub(crate) fn with_conversation(kb: &KnowledgeBase, store: ConversationStore) -> Self {
        let mut cached = Vec::new();
        let kb_names = [
            "kb-01-platform-rules",
            "kb-02-seo-keywords",
            "kb-03-case-studies",
            "kb-04-terminology",
            "kb-05-compliance-rules",
            "kb-06-product-catalog",
        ];
        for name in &kb_names {
            cached.extend(kb.query(name, None, None, None, 100));
        }
        Self {
            cached_kb: cached,
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

        // Multi-turn conversation support: inject conversation history if thread_id is provided
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

        // Step 1: IntentClassifier
        {
            let start = Instant::now();
            let step = match step_intent_classifier(&mut ctx, llm).await {
                Ok(s) => s,
                Err(e) => {
                    return build_partial_response(
                        request_id,
                        params.agent_type,
                        steps,
                        format!("IntentClassifier failed: {:?}", e),
                    );
                }
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Step 2: InfoCollector
        {
            let start = Instant::now();
            let step = match step_info_collector(&mut ctx, llm).await {
                Ok(s) => s,
                Err(e) => {
                    return build_partial_response(
                        request_id,
                        params.agent_type,
                        steps,
                        format!("InfoCollector failed: {:?}", e),
                    );
                }
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Read intent and branch
        let intent = ctx
            .get("intent_classifier")
            .and_then(|v| v.get("intent"))
            .and_then(|v| v.as_str())
            .unwrap_or("A")
            .to_string();

        let branch_result = match intent.as_str() {
            "A" => self.run_intent_a(params, llm, &mut ctx, &mut steps).await,
            "B" => self.run_intent_b(params, llm, &mut ctx, &mut steps).await,
            "C" => self.run_intent_c(params, llm, &mut ctx, &mut steps).await,
            "D" => self.run_intent_d(params, llm, &mut ctx, &mut steps).await,
            "E" => self.run_intent_e(params, llm, &mut ctx, &mut steps).await,
            _ => self.run_intent_f(params, llm, &mut ctx, &mut steps).await,
        };

        if let Err(e) = branch_result {
            return build_partial_response(
                request_id,
                params.agent_type,
                steps,
                format!("Intent {} branch failed: {:?}", intent, e),
            );
        }

        let markdown = ctx
            .get("final_markdown")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        // Save assistant response to conversation store for multi-turn continuity
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
                "agent": "ListingBot Pro",
                "intent": intent,
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

    // Intent A: Generate new listing (byte-for-byte equivalent to original steps 3-8)
    async fn run_intent_a(
        &self,
        params: &EcommerceAgentSubmitParams,
        llm: &LlmClient,
        ctx: &mut WorkflowContext,
        steps: &mut Vec<super::types::EcommerceAgentStepResult>,
    ) -> Result<(), JSONRPCErrorError> {
        // Step 3: KnowledgeRetrieval
        {
            let start = Instant::now();
            let step = step_knowledge_retrieval(ctx, &self.cached_kb);
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Step 4: ListingGenerator
        {
            let start = Instant::now();
            let step = match step_listing_generator(ctx, llm).await {
                Ok(s) => s,
                Err(e) => return Err(e),
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Step 5: ComplianceChecker
        {
            let start = Instant::now();
            let step = match step_compliance_checker(ctx, llm).await {
                Ok(s) => s,
                Err(e) => return Err(e),
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Step 6: SeoScorer
        {
            let start = Instant::now();
            let step = match step_seo_scorer(ctx, llm).await {
                Ok(s) => s,
                Err(e) => return Err(e),
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Step 7: Localizer
        {
            let start = Instant::now();
            let step = match step_localizer(ctx, llm).await {
                Ok(s) => s,
                Err(e) => return Err(e),
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // Step 8: OutputAssembler
        {
            let start = Instant::now();
            let step = step_output_assembler(ctx, params);
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        Ok(())
    }

    // Intent B: Optimize existing listing
    async fn run_intent_b(
        &self,
        _params: &EcommerceAgentSubmitParams,
        llm: &LlmClient,
        ctx: &mut WorkflowContext,
        steps: &mut Vec<super::types::EcommerceAgentStepResult>,
    ) -> Result<(), JSONRPCErrorError> {
        // KnowledgeRetrieval
        {
            let start = Instant::now();
            let step = step_knowledge_retrieval(ctx, &self.cached_kb);
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // ListingOptimizer
        {
            let start = Instant::now();
            let step = match step_listing_optimizer(ctx, llm).await {
                Ok(s) => s,
                Err(e) => return Err(e),
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // OutputAssembler (branch-aware)
        {
            let start = Instant::now();
            let step = step_output_assembler_branch(ctx, "B");
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        Ok(())
    }

    // Intent C: Standalone translation
    async fn run_intent_c(
        &self,
        _params: &EcommerceAgentSubmitParams,
        llm: &LlmClient,
        ctx: &mut WorkflowContext,
        steps: &mut Vec<super::types::EcommerceAgentStepResult>,
    ) -> Result<(), JSONRPCErrorError> {
        // KnowledgeRetrieval
        {
            let start = Instant::now();
            let step = step_knowledge_retrieval(ctx, &self.cached_kb);
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // StandaloneTranslator
        {
            let start = Instant::now();
            let step = match step_standalone_translator(ctx, llm).await {
                Ok(s) => s,
                Err(e) => return Err(e),
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // OutputAssembler (branch-aware)
        {
            let start = Instant::now();
            let step = step_output_assembler_branch(ctx, "C");
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        Ok(())
    }

    // Intent D: Standalone compliance check
    async fn run_intent_d(
        &self,
        _params: &EcommerceAgentSubmitParams,
        llm: &LlmClient,
        ctx: &mut WorkflowContext,
        steps: &mut Vec<super::types::EcommerceAgentStepResult>,
    ) -> Result<(), JSONRPCErrorError> {
        // StandaloneCompliance
        {
            let start = Instant::now();
            let step = match step_standalone_compliance(ctx, llm).await {
                Ok(s) => s,
                Err(e) => return Err(e),
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // OutputAssembler (branch-aware)
        {
            let start = Instant::now();
            let step = step_output_assembler_branch(ctx, "D");
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        Ok(())
    }

    // Intent E: SEO advisor
    async fn run_intent_e(
        &self,
        _params: &EcommerceAgentSubmitParams,
        llm: &LlmClient,
        ctx: &mut WorkflowContext,
        steps: &mut Vec<super::types::EcommerceAgentStepResult>,
    ) -> Result<(), JSONRPCErrorError> {
        // KnowledgeRetrieval
        {
            let start = Instant::now();
            let step = step_knowledge_retrieval(ctx, &self.cached_kb);
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // SeoAdvisor
        {
            let start = Instant::now();
            let step = match step_seo_advisor(ctx, llm).await {
                Ok(s) => s,
                Err(e) => return Err(e),
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // OutputAssembler (branch-aware)
        {
            let start = Instant::now();
            let step = step_output_assembler_branch(ctx, "E");
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        Ok(())
    }

    // Intent F: General chat
    async fn run_intent_f(
        &self,
        _params: &EcommerceAgentSubmitParams,
        llm: &LlmClient,
        ctx: &mut WorkflowContext,
        steps: &mut Vec<super::types::EcommerceAgentStepResult>,
    ) -> Result<(), JSONRPCErrorError> {
        // GeneralChat
        {
            let start = Instant::now();
            let step = match step_general_chat(ctx, llm).await {
                Ok(s) => s,
                Err(e) => return Err(e),
            };
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        // OutputAssembler (branch-aware)
        {
            let start = Instant::now();
            let step = step_output_assembler_branch(ctx, "F");
            let duration = start.elapsed().as_millis() as i64;
            steps.push(make_step_result(&step, duration));
        }

        Ok(())
    }
}

// ---------------------------------------------------------------------------
// Step implementations
// ---------------------------------------------------------------------------

async fn step_intent_classifier(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let user_input = ctx_str(ctx, "user_input");
    let conv_history = ctx
        .get("conversation_history")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let user_message = if !conv_history.is_empty() {
        format!("用户消息: {}\n\n{}\n\n请根据完整的对话历史判断用户意图。", user_input, conv_history)
    } else {
        format!("用户消息: {}", user_input)
    };
    let response = llm.chat("gpt-4o-mini", INTENT_CLASSIFIER_SYSTEM, &user_message).await?;

    let parsed: Value = serde_json::from_str(&response).unwrap_or_else(|_| {
        json!({
            "intent": "A",
            "confidence": 0.5,
            "sub_intent": "parse_fallback",
            "detected_info": {},
            "raw_response": response
        })
    });

    ctx.insert("intent_classifier".to_string(), parsed.clone());

    Ok(StepOutput {
        step_name: "intent_classifier".to_string(),
        model: "gpt-4o-mini".to_string(),
        output: parsed,
    })
}

async fn step_info_collector(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let user_input = ctx_str(ctx, "user_input");
    let detected = ctx
        .get("intent_classifier")
        .and_then(|v| v.get("detected_info"))
        .cloned()
        .unwrap_or(json!({}));

    let user_message = format!(
        "用户消息: {}\n\n已提取到的信息: {}",
        user_input,
        detected
    );
    let response = llm
        .chat("gpt-4o-mini", INFO_COLLECTOR_SYSTEM, &user_message)
        .await?;

    let parsed: Value = serde_json::from_str(&response).unwrap_or_else(|_| {
        json!({
            "status": "complete",
            "product_info": {
                "product_name": ctx_str(ctx, "user_input"),
                "key_features": ["feature_1", "feature_2"],
                "specifications": {},
                "platform": ctx_str(ctx, "platform"),
                "market": [ctx_str(ctx, "market")],
                "brand_name": "Generic",
                "target_audience": null,
                "price_range": null,
                "competitor_asin": null,
                "unique_selling_point": null,
                "certifications": [],
                "package_contents": null
            }
        })
    });

    ctx.insert("info_collector".to_string(), parsed.clone());

    // Also store product_info for downstream steps
    if let Some(pi) = parsed.get("product_info") {
        ctx.insert("product_info".to_string(), pi.clone());
    }

    Ok(StepOutput {
        step_name: "info_collector".to_string(),
        model: "gpt-4o-mini".to_string(),
        output: parsed,
    })
}

fn step_knowledge_retrieval(
    ctx: &mut WorkflowContext,
    cached_kb: &[KbEntry],
) -> StepOutput {
    let platform = ctx_str(ctx, "platform");
    let market = ctx_str(ctx, "market");

    let matched: Vec<String> = cached_kb
        .iter()
        .filter(|e| {
            let p_ok = platform.is_empty()
                || e.platform.as_deref().map_or(true, |p| p.contains(&platform) || platform.contains(p));
            let m_ok = market.is_empty()
                || e.market.as_deref().map_or(true, |m| m.contains(&market) || market.contains(m));
            p_ok && m_ok
        })
        .take(20)
        .map(|e| e.content.clone())
        .collect();

    let combined = if matched.is_empty() {
        let fallback: Vec<String> = cached_kb.iter().take(10).map(|e| e.content.clone()).collect();
        fallback.join("\n---\n")
    } else {
        matched.join("\n---\n")
    };

    ctx.insert("kb_results".to_string(), json!(combined));

    StepOutput {
        step_name: "knowledge_retrieval".to_string(),
        model: "".to_string(),
        output: json!({
            "entries_count": cached_kb.len(),
            "matched_count": matched.len(),
            "combined_length": combined.len(),
        }),
    }
}

async fn step_listing_generator(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let product_info = ctx
        .get("product_info")
        .map(|v| v.to_string())
        .unwrap_or_default();
    let kb_results = ctx
        .get("kb_results")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let platform = ctx_str(ctx, "platform");
    let market = ctx_str(ctx, "market");

    let user_message = format!(
        "产品信息: {}\n\n平台规则参考: {}\n\nSEO关键词数据: {}\n\n爆款案例参考: {}\n\n品类专业术语: {}\n\n目标平台: {}\n目标市场: {}",
        product_info,
        kb_results.chars().take(1000).collect::<String>(),
        kb_results.chars().take(500).collect::<String>(),
        kb_results.chars().take(500).collect::<String>(),
        kb_results.chars().take(300).collect::<String>(),
        platform,
        market,
    );

    let response = llm
        .chat("claude-3-5-sonnet-20241022", LISTING_GENERATOR_SYSTEM, &user_message)
        .await?;

    let output = json!({
        "listing_markdown": response,
        "length": response.len(),
    });

    ctx.insert("listing_generator".to_string(), json!(response));

    Ok(StepOutput {
        step_name: "listing_generator".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        output,
    })
}

async fn step_compliance_checker(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let listing = ctx_str(ctx, "listing_generator");
    let platform = ctx_str(ctx, "platform");
    let market = ctx_str(ctx, "market");
    let kb_results = ctx_str(ctx, "kb_results");

    let user_message = format!(
        "待检查的 Listing 内容:\n{}\n\n目标平台: {}\n目标市场: {}\n\n合规规则参考:\n{}",
        listing,
        platform,
        market,
        kb_results.chars().take(500).collect::<String>(),
    );

    let response = llm
        .chat("gpt-4o-mini", COMPLIANCE_CHECKER_SYSTEM, &user_message)
        .await?;

    let output = json!({
        "compliance_report": response,
    });

    ctx.insert("compliance_checker".to_string(), json!(response));

    Ok(StepOutput {
        step_name: "compliance_checker".to_string(),
        model: "gpt-4o-mini".to_string(),
        output,
    })
}

async fn step_seo_scorer(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let listing = ctx_str(ctx, "listing_generator");
    let kb_results = ctx_str(ctx, "kb_results");

    let user_message = format!(
        "Listing 内容:\n{}\n\nSEO关键词数据:\n{}",
        listing,
        kb_results.chars().take(500).collect::<String>(),
    );

    let response = llm
        .chat("gpt-4o-mini", SEO_SCORER_SYSTEM, &user_message)
        .await?;

    let output = json!({
        "seo_report": response,
    });

    ctx.insert("seo_scorer".to_string(), json!(response));

    Ok(StepOutput {
        step_name: "seo_scorer".to_string(),
        model: "gpt-4o-mini".to_string(),
        output,
    })
}

async fn step_localizer(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let listing = ctx_str(ctx, "listing_generator");
    let market = ctx_str(ctx, "market");
    let kb_results = ctx_str(ctx, "kb_results");

    let user_message = format!(
        "英文 Listing 原文:\n{}\n\n目标市场列表:\n{}\n\n多语言营销表达参考:\n{}\n\n品类专业术语参考:\n{}",
        listing,
        market,
        kb_results.chars().take(500).collect::<String>(),
        kb_results.chars().take(300).collect::<String>(),
    );

    let response = llm
        .chat("gpt-4o", LOCALIZER_SYSTEM, &user_message)
        .await?;

    let output = json!({
        "localized_content": response,
    });

    ctx.insert("localizer".to_string(), json!(response));

    Ok(StepOutput {
        step_name: "localizer".to_string(),
        model: "gpt-4o".to_string(),
        output,
    })
}

async fn step_listing_optimizer(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let user_input = ctx_str(ctx, "user_input");
    let kb_results = ctx_str(ctx, "kb_results");

    let user_message = format!(
        "## 用户提供的现有 Listing\n{}\n\n## 平台规则参考\n{}\n\n## SEO关键词数据\n{}\n\n## 爆款案例参考\n{}\n\n## 违禁词库\n{}",
        user_input,
        kb_results.chars().take(1000).collect::<String>(),
        kb_results.chars().take(500).collect::<String>(),
        kb_results.chars().take(500).collect::<String>(),
        kb_results.chars().take(500).collect::<String>(),
    );

    let response = llm
        .chat("claude-3-5-sonnet-20241022", LISTING_OPTIMIZER_SYSTEM, &user_message)
        .await?;

    ctx.insert("listing_optimizer".to_string(), json!(response));

    Ok(StepOutput {
        step_name: "listing_optimizer".to_string(),
        model: "claude-3-5-sonnet-20241022".to_string(),
        output: json!({ "optimized_listing": response, "length": response.len() }),
    })
}

async fn step_standalone_translator(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let user_input = ctx_str(ctx, "user_input");
    let market = ctx_str(ctx, "market");

    let user_message = format!(
        "## Listing 原文\n{}\n\n## 目标市场\n{}",
        user_input, market,
    );

    let response = llm
        .chat("gpt-4o", STANDALONE_TRANSLATOR_SYSTEM, &user_message)
        .await?;

    ctx.insert("standalone_translator".to_string(), json!(response));

    Ok(StepOutput {
        step_name: "standalone_translator".to_string(),
        model: "gpt-4o".to_string(),
        output: json!({ "translated_content": response, "length": response.len() }),
    })
}

async fn step_standalone_compliance(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let user_input = ctx_str(ctx, "user_input");

    let user_message = format!("## 用户提供的 Listing 文本\n{}", user_input);

    let response = llm
        .chat("gpt-4o-mini", STANDALONE_COMPLIANCE_SYSTEM, &user_message)
        .await?;

    ctx.insert("standalone_compliance".to_string(), json!(response));

    Ok(StepOutput {
        step_name: "standalone_compliance".to_string(),
        model: "gpt-4o-mini".to_string(),
        output: json!({ "compliance_report": response }),
    })
}

async fn step_seo_advisor(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let user_input = ctx_str(ctx, "user_input");
    let kb_results = ctx_str(ctx, "kb_results");

    let user_message = format!(
        "## 用户描述\n{}\n\n## SEO关键词数据\n{}",
        user_input,
        kb_results.chars().take(1000).collect::<String>(),
    );

    let response = llm
        .chat("gpt-4o-mini", SEO_ADVISOR_SYSTEM, &user_message)
        .await?;

    ctx.insert("seo_advisor".to_string(), json!(response));

    Ok(StepOutput {
        step_name: "seo_advisor".to_string(),
        model: "gpt-4o-mini".to_string(),
        output: json!({ "seo_report": response, "length": response.len() }),
    })
}

async fn step_general_chat(
    ctx: &mut WorkflowContext,
    llm: &LlmClient,
) -> Result<StepOutput, JSONRPCErrorError> {
    let user_input = ctx_str(ctx, "user_input");

    let response = llm
        .chat("gpt-4o-mini", GENERAL_CHAT_SYSTEM, &user_input)
        .await?;

    ctx.insert("general_chat".to_string(), json!(response));

    Ok(StepOutput {
        step_name: "general_chat".to_string(),
        model: "gpt-4o-mini".to_string(),
        output: json!({ "chat_response": response }),
    })
}

fn step_output_assembler_branch(
    ctx: &mut WorkflowContext,
    intent: &str,
) -> StepOutput {
    let markdown = match intent {
        "B" => {
            let optimizer = ctx_str(ctx, "listing_optimizer");
            format!("# ListingBot Pro — 优化完成\n\n---\n\n{optimizer}\n")
        }
        "C" => {
            let translated = ctx_str(ctx, "standalone_translator");
            format!("# ListingBot Pro — 翻译完成\n\n---\n\n{translated}\n")
        }
        "D" => {
            let compliance = ctx_str(ctx, "standalone_compliance");
            format!("# ListingBot Pro — 合规检查完成\n\n---\n\n{compliance}\n")
        }
        "E" => {
            let seo = ctx_str(ctx, "seo_advisor");
            format!("# ListingBot Pro — SEO 分析报告\n\n---\n\n{seo}\n")
        }
        _ => {
            let chat = ctx_str(ctx, "general_chat");
            format!("# ListingBot Pro\n\n---\n\n{chat}\n")
        }
    };

    ctx.insert("final_markdown".to_string(), json!(markdown));

    StepOutput {
        step_name: "output_assembler".to_string(),
        model: "".to_string(),
        output: json!({ "output_length": markdown.len() }),
    }
}

fn step_output_assembler(
    ctx: &mut WorkflowContext,
    _params: &EcommerceAgentSubmitParams,
) -> StepOutput {
    let listing = ctx_str(ctx, "listing_generator");
    let compliance = ctx_str(ctx, "compliance_checker");
    let seo = ctx_str(ctx, "seo_scorer");
    let localized = ctx_str(ctx, "localizer");

    let markdown = format!(
        r#"# ListingBot Pro — 生成完成 ✅

---

{listing}

---

{compliance}

---

{seo}

---

{localized}

---

## 📌 下一步操作指引

1. **检查合规报告** — 如有 🔴 标记的项目，请务必修改后再上架
2. **参考SEO评分** — 分数低于75分的建议根据优化建议修改
3. **多语言版本** — 带有 ⚠️ 标记的表达建议找母语者校验
4. **上传到卖家后台** — 直接复制对应内容到Seller Central

💬 **需要修改？** 直接告诉我要调整哪个部分，例如：
- "标题再简短一些"
- "Bullet第三条换个卖点"
- "日语版的语气再正式一些"
- "帮我增加韩语版本"
"#,
        listing = listing,
        compliance = compliance,
        seo = seo,
        localized = localized,
    );

    ctx.insert("final_markdown".to_string(), json!(markdown));

    StepOutput {
        step_name: "output_assembler".to_string(),
        model: "".to_string(),
        output: json!({
            "output_length": markdown.len(),
        }),
    }
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn ctx_str(ctx: &WorkflowContext, key: &str) -> String {
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
