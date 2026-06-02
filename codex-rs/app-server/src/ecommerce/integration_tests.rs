use codex_app_server_protocol::EcommerceAgentSubmitParams;
use codex_app_server_protocol::EcommerceAgentType;

use super::llm_client::{LlmClient, LlmConfig};
use super::listing_bot::ListingBot;
use super::customer_bot::CustomerBot;
use super::knowledge_base::KnowledgeBase;
use super::conversation_store::ConversationStore;

#[test]
fn test_config_resolution() {
    let config = LlmConfig::resolve();
    match config {
        Some(cfg) => println!(
            "Config resolved: base_url={}, protocol={:?}, model={}, api_key={}",
            cfg.base_url,
            cfg.protocol,
            cfg.default_model,
            cfg.api_key.as_ref().map(|k| {
                let s = k.as_str();
                format!("{}...", &s[..8.min(s.len())])
            }).unwrap_or_else(|| "none".to_string()),
        ),
        None => panic!("Config not resolved!"),
    }

    // Now verify it actually works
    let _client = LlmClient::new();
    println!("LlmClient created successfully");
}

fn make_kb() -> KnowledgeBase {
    let kb_dir = std::env::var("ECOMMERCE_KB_DIR")
        .unwrap_or_else(|_| "/Users/yuqei/AI-Agent-Shopping/knowledge-base".to_string());
    let mut kb = KnowledgeBase::new(&kb_dir);
    kb.load_all();
    kb
}

fn make_params(agent_type: EcommerceAgentType, user_input: &str) -> EcommerceAgentSubmitParams {
    EcommerceAgentSubmitParams {
        agent_type,
        user_input: user_input.to_string(),
        platform: Some("Amazon".to_string()),
        market: Some("US".to_string()),
        context: None,
        thread_id: None,
    }
}

#[test]
fn test_knowledge_base_loading() {
    let kb = make_kb();

    // KB should have loaded the CS entries
    let faq_entries = kb.query("kb-cs-01-faq", None, None, None, 10);
    assert!(!faq_entries.is_empty(), "KB-CS-01 FAQ should have entries");

    let product_entries = kb.query("kb-cs-02-product-info", None, None, None, 10);
    assert!(!product_entries.is_empty(), "KB-CS-02 Product Info should have entries");

    let shipping_entries = kb.query("kb-cs-03-shipping", None, None, None, 10);
    assert!(!shipping_entries.is_empty(), "KB-CS-03 Shipping should have entries");
}

#[tokio::test]
async fn test_customer_bot_routing_structure() {
    let kb = make_kb();
    let llm = LlmClient::new();
    let params = make_params(
        EcommerceAgentType::CustomerBot,
        "这款蓝牙耳机支持 iPhone 15 吗？",
    );

    let bot = CustomerBot::new(&kb);
    let response = bot.run(&params, &llm).await;

    assert!(!response.request_id.is_empty());
    assert_eq!(response.agent_type, EcommerceAgentType::CustomerBot);
    assert!(!response.intermediate_steps.is_empty());

    // First step should always be lang_intent_detector
    assert_eq!(
        response.intermediate_steps[0].step_name,
        "lang_intent_detector"
    );
}

#[tokio::test]
async fn test_listing_bot_intent_a_routing_structure() {
    let kb = make_kb();
    let llm = LlmClient::new();
    let params = make_params(
        EcommerceAgentType::ListingBot,
        "帮我为一款无线蓝牙耳机生成 Amazon 产品页面文案",
    );

    let bot = ListingBot::new(&kb);
    let response = bot.run(&params, &llm).await;

    // Response should always have these fields populated
    assert!(!response.request_id.is_empty());
    assert_eq!(response.agent_type, EcommerceAgentType::ListingBot);

    // Debug: print error if steps are empty
    if let Some(ref err) = response.error {
        // Check if it's a timeout (524) - that's a transient API issue, not a code bug
        if err.contains("524") || err.contains("timeout") {
            println!("Intent A skipped due to API timeout: {}", err);
            return;
        }
        panic!("Intent classifier failed with error: {}", err);
    }

    // Even if API fails, we should have intermediate steps
    assert!(
        !response.intermediate_steps.is_empty(),
        "No steps recorded. Status: {}, Error: {:?}, Output: {:?}",
        response.status,
        response.error,
        response.output_markdown.as_ref().map(|s| &s[..100.min(s.len())]),
    );

    // Verify intent A flow was executed
    let step_names: Vec<&str> = response.intermediate_steps
        .iter()
        .map(|s| s.step_name.as_str())
        .collect();
    assert!(step_names.contains(&"intent_classifier"));
    assert!(step_names.contains(&"listing_generator"));
}

#[tokio::test]
async fn test_customer_bot_escalation_detection() {
    let kb = make_kb();
    let llm = LlmClient::new();
    let params = make_params(
        EcommerceAgentType::CustomerBot,
        "我非常愤怒！你们的产品完全不能用，我要投诉！我要退款！",
    );

    let bot = CustomerBot::new(&kb);
    let response = bot.run(&params, &llm).await;

    // Either escalated or completed, both are valid outcomes
    // Transient API errors (429, 524) are not code bugs
    if let Some(ref err) = response.error {
        if err.contains("429") || err.contains("524") || err.contains("timeout") {
            println!("Escalation test skipped due to API rate limit/timeout: {}", err);
            return;
        }
    }

    assert!(
        response.status == "escalated" || response.status == "completed",
        "Unexpected status: '{}', Error: {:?}, Output: {:?}",
        response.status,
        response.error,
        response.output_markdown.as_ref().map(|s| &s[..200.min(s.len())]),
    );
}

/// Test ListingBot Intent B-F routing by checking step names
#[tokio::test]
async fn test_listing_bot_intent_variants() {
    let kb = make_kb();
    let llm = LlmClient::new();

    let test_cases = vec![
        ("帮我优化这条 Listing：【标题】无线蓝牙耳机", "intent_classifier"),
        ("把这条 Listing 翻译成日语", "intent_classifier"),
        ("帮我检查这条 Listing 是否合规", "intent_classifier"),
        ("分析这条 Listing 的 SEO", "intent_classifier"),
        ("你好，请问你们怎么样？", "intent_classifier"),
    ];

    for (input, expected_first_step) in test_cases {
        let params = make_params(EcommerceAgentType::ListingBot, input);
        let bot = ListingBot::new(&kb);
        let response = bot.run(&params, &llm).await;

        // Handle rate limit / timeout gracefully
        if let Some(ref err) = response.error {
            if err.contains("429") || err.contains("524") || err.contains("timeout") {
                println!("Skipping variant '{}' due to API rate limit/timeout", input);
                // Brief pause before retry
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                continue;
            }
        }

        assert!(
            !response.intermediate_steps.is_empty(),
            "No steps for input '{}'. Error: {:?}, Status: {}",
            input,
            response.error,
            response.status
        );
        assert_eq!(
            response.intermediate_steps[0].step_name,
            expected_first_step,
            "Failed for input: {}",
            input
        );

        // Brief pause between calls to avoid rate limits
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

/// Test ConversationStore basic functionality
#[test]
fn test_conversation_store_basic() {
    let store = ConversationStore::new(None);

    store.add_message("thread-1", "user", "Hello");
    store.add_message("thread-1", "assistant", "Hi there!");
    store.add_message("thread-1", "user", "How are you?");

    let context = store.get_context("thread-1", 10).unwrap();
    assert!(context.contains("Hello"));
    assert!(context.contains("Hi there!"));
    assert!(context.contains("How are you?"));
    assert!(context.contains("用户"));
    assert!(context.contains("助手"));
}

/// Test ConversationStore thread isolation
#[test]
fn test_conversation_store_thread_isolation() {
    let store = ConversationStore::new(None);

    store.add_message("thread-1", "user", "Message for thread 1");
    store.add_message("thread-2", "user", "Message for thread 2");

    let ctx1 = store.get_context("thread-1", 10).unwrap();
    let ctx2 = store.get_context("thread-2", 10).unwrap();

    assert!(ctx1.contains("Message for thread 1"));
    assert!(!ctx1.contains("Message for thread 2"));
    assert!(ctx2.contains("Message for thread 2"));
    assert!(!ctx2.contains("Message for thread 1"));
}

/// Test ConversationStore max_turns trimming
#[test]
fn test_conversation_store_max_turns() {
    let store = ConversationStore::new(None);

    for i in 0..20 {
        store.add_message("thread-1", "user", &format!("Message {}", i));
    }

    let context = store.get_context("thread-1", 5).unwrap();
    assert!(!context.contains("Message 0"));
    assert!(context.contains("Message 19"));
}

/// Test ConversationStore disk persistence
#[test]
fn test_conversation_store_disk_persistence() {
    let temp_dir = std::env::temp_dir().join("conv-store-test");
    let _ = std::fs::remove_dir_all(&temp_dir);

    // Create store, add messages
    {
        let store = ConversationStore::new(Some(temp_dir.clone()));
        store.add_message("persist-thread", "user", "Persistent message");
        store.add_message("persist-thread", "assistant", "Persistent response");
    }

    // Create new store, should load from disk
    {
        let store = ConversationStore::new(Some(temp_dir.clone()));
        let context = store.get_context("persist-thread", 10);
        assert!(context.is_some());
        let ctx = context.unwrap();
        assert!(ctx.contains("Persistent message"));
        assert!(ctx.contains("Persistent response"));
    }

    let _ = std::fs::remove_dir_all(&temp_dir);
}

/// Test ConversationStore clear
#[test]
fn test_conversation_store_clear() {
    let store = ConversationStore::new(None);
    store.add_message("thread-1", "user", "Test message");

    store.clear("thread-1");

    let context = store.get_context("thread-1", 10);
    assert!(context.is_none());
}
