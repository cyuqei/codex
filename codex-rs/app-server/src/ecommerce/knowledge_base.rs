use std::collections::HashMap;

use serde_json::Value;

#[derive(Clone, Debug)]
pub(crate) struct KbEntry {
    pub id: String,
    pub content: String,
    pub platform: Option<String>,
    pub market: Option<String>,
    pub category: Option<String>,
}

pub(crate) struct KnowledgeBase {
    kb_entries: HashMap<String, Vec<KbEntry>>,
    kb_dir: String,
    /// TF-IDF index: KB name → vec of HashMap<token, tfidf_weight>.
    tfidf_index: HashMap<String, Vec<HashMap<String, f64>>>,
    /// Global IDF table: token → inverse document frequency.
    idf: HashMap<String, f64>,
}

impl KnowledgeBase {
    pub fn new(kb_dir: &str) -> Self {
        Self {
            kb_entries: HashMap::new(),
            kb_dir: kb_dir.to_string(),
            tfidf_index: HashMap::new(),
            idf: HashMap::new(),
        }
    }

    pub fn load_all(&mut self) {
        let kb_files = [
            "kb-01-platform-rules",
            "kb-02-seo-keywords",
            "kb-03-case-studies",
            "kb-04-terminology",
            "kb-05-compliance-rules",
            "kb-06-product-catalog",
            "kb-cs-01-faq",
            "kb-cs-02-product-info",
            "kb-cs-03-shipping",
            "kb-cs-04-returns",
            "kb-cs-05-scripts",
            "kb-ads-01-platform-rules",
        ];
        for name in &kb_files {
            let path = format!("{}/{}.json", self.kb_dir, name);
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Ok(entries) = Self::parse_kb_file(content) {
                    self.build_tf_idf(name, &entries);
                    self.kb_entries.insert(name.to_string(), entries);
                }
            }
        }
    }

    fn parse_kb_file(content: String) -> Result<Vec<KbEntry>, String> {
        let value: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;
        let entries: Vec<KbEntry> = match &value {
            Value::Array(arr) => arr
                .iter()
                .filter_map(|item| Self::json_to_kb_entry(item))
                .collect(),
            _ => vec![],
        };
        Ok(entries)
    }

    fn json_to_kb_entry(item: &Value) -> Option<KbEntry> {
        let id = item.get("id")?.as_str()?.to_string();
        let content = serde_json::to_string_pretty(item).ok()?;
        let platform = item.get("platform").and_then(|v| v.as_str()).map(|s| s.to_string());
        let market = item.get("market").and_then(|v| v.as_str()).map(|s| s.to_string());
        let category = item.get("category").and_then(|v| v.as_str()).map(|s| s.to_string());
        Some(KbEntry {
            id,
            content,
            platform,
            market,
            category,
        })
    }

    fn tokenize(text: &str) -> Vec<String> {
        text.to_lowercase()
            .split(|c: char| !c.is_alphanumeric() && c != '_')
            .filter(|t| t.len() > 1)
            .map(|s| s.to_string())
            .collect()
    }

    fn build_tf_idf(&mut self, kb_name: &str, entries: &[KbEntry]) {
        let mut global_df: HashMap<String, usize> = HashMap::new();
        let n_docs = entries.len() as f64;
        let mut kb_weights: Vec<HashMap<String, f64>> = Vec::new();

        for entry in entries {
            let text = format!(
                "{} {} {} {}",
                entry.content,
                entry.platform.as_deref().unwrap_or(""),
                entry.market.as_deref().unwrap_or(""),
                entry.category.as_deref().unwrap_or(""),
            );
            let tokens = Self::tokenize(&text);
            let mut term_freq: HashMap<String, usize> = HashMap::new();
            for token in &tokens {
                *term_freq.entry(token.clone()).or_insert(0) += 1;
            }
            let max_tf = term_freq.values().max().copied().unwrap_or(1) as f64;
            let tf_weights: HashMap<String, f64> = term_freq
                .into_iter()
                .map(|(t, c)| (t, c as f64 / max_tf))
                .collect();

            for t in tf_weights.keys() {
                *global_df.entry(t.clone()).or_insert(0) += 1;
            }

            kb_weights.push(tf_weights);
        }

        // Update global IDF
        for (token, count) in &global_df {
            let idf_val = (n_docs / (1.0 + *count as f64)).ln() + 1.0;
            *self.idf.entry(token.clone()).or_insert(0.0) += idf_val;
        }

        // Store TF-IDF weighted vectors
        let mut tfidf_docs: Vec<HashMap<String, f64>> = Vec::new();
        for doc_tf in &kb_weights {
            let mut doc_vec: HashMap<String, f64> = HashMap::new();
            for (t, tf) in doc_tf {
                let idf_val = self.idf.get(t).copied().unwrap_or(1.0);
                doc_vec.insert(t.clone(), tf * idf_val);
            }
            tfidf_docs.push(doc_vec);
        }

        self.tfidf_index.insert(kb_name.to_string(), tfidf_docs);
    }

    fn score_document(&self, query_tokens: &[String], doc_weights: &HashMap<String, f64>) -> f64 {
        let mut query_tf: HashMap<String, f64> = HashMap::new();
        for t in query_tokens {
            *query_tf.entry(t.clone()).or_insert(0.0) += 1.0;
        }
        let query_max = query_tf.values().cloned().fold(0.0, f64::max);
        if query_max == 0.0 {
            return 0.0;
        }

        let mut query_vec: HashMap<String, f64> = HashMap::new();
        for (t, c) in &query_tf {
            let idf = self.idf.get(t).copied().unwrap_or(1.0);
            query_vec.insert(t.clone(), (c / query_max) * idf);
        }

        let mut dot = 0.0;
        let mut norm_q = 0.0;
        let mut norm_d = 0.0;
        for (t, qv) in &query_vec {
            norm_q += qv * qv;
            if let Some(dv) = doc_weights.get(t) {
                dot += qv * dv;
            }
        }
        for dv in doc_weights.values() {
            norm_d += dv * dv;
        }

        if norm_q == 0.0 || norm_d == 0.0 {
            return 0.0;
        }
        dot / (norm_q.sqrt() * norm_d.sqrt())
    }

    /// Metadata filter score.
    fn meta_match(e: &KbEntry, platform: Option<&str>, market: Option<&str>, category: Option<&str>) -> f64 {
        let mut score = 0.0;
        if let Some(p) = platform {
            if e.platform.as_deref() == Some(p) { score += 2.0; }
            else if e.platform.is_some() {}
            else { score += 1.0; }
        }
        if let Some(m) = market {
            if e.market.as_deref() == Some(m) { score += 2.0; }
            else if e.market.is_some() {}
            else { score += 1.0; }
        }
        if let Some(c) = category {
            if e.category.as_deref() == Some(c) { score += 2.0; }
            else if e.category.is_some() {}
            else { score += 1.0; }
        }
        score
    }

    pub fn query(
        &self,
        kb_name: &str,
        platform: Option<&str>,
        market: Option<&str>,
        category: Option<&str>,
        top_k: usize,
    ) -> Vec<KbEntry> {
        let Some(entries) = self.kb_entries.get(kb_name) else {
            return vec![];
        };
        self.query_internal(kb_name, entries, platform, market, category, top_k)
    }

    /// Semantic text query: ranks entries by TF-IDF cosine similarity.
    pub fn semantic_query(&self, kb_name: &str, query: &str, top_k: usize) -> Vec<KbEntry> {
        let Some(entries) = self.kb_entries.get(kb_name) else {
            return vec![];
        };
        let query_tokens = Self::tokenize(query);
        let Some(doc_weights) = self.tfidf_index.get(kb_name) else {
            return self.keyword_fallback(entries, None, None, None, top_k);
        };

        let mut scores: Vec<(usize, f64)> = entries
            .iter()
            .enumerate()
            .map(|(i, _)| (i, self.score_document(&query_tokens, &doc_weights[i])))
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.into_iter().take(top_k).map(|(i, _)| entries[i].clone()).collect()
    }

    fn query_internal(
        &self,
        kb_name: &str,
        entries: &[KbEntry],
        platform: Option<&str>,
        market: Option<&str>,
        category: Option<&str>,
        top_k: usize,
    ) -> Vec<KbEntry> {
        let Some(doc_weights) = self.tfidf_index.get(kb_name) else {
            return self.keyword_fallback(entries, platform, market, category, top_k);
        };

        let query_text = format!(
            "{} {} {}",
            platform.unwrap_or(""),
            market.unwrap_or(""),
            category.unwrap_or(""),
        );
        let query_tokens = Self::tokenize(&query_text);

        let mut scores: Vec<(usize, f64)> = entries
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let meta = Self::meta_match(e, platform, market, category);
                let semantic = if query_tokens.is_empty() {
                    1.0
                } else {
                    self.score_document(&query_tokens, &doc_weights[i])
                };
                (i, meta + semantic)
            })
            .collect();

        scores.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
        scores.into_iter().take(top_k).map(|(i, _)| entries[i].clone()).collect()
    }

    fn keyword_fallback(
        &self,
        entries: &[KbEntry],
        platform: Option<&str>,
        market: Option<&str>,
        category: Option<&str>,
        top_k: usize,
    ) -> Vec<KbEntry> {
        let filtered: Vec<KbEntry> = entries
            .iter()
            .filter(|e| {
                platform.map_or(true, |p| e.platform.as_deref() == Some(p))
                    && market.map_or(true, |m| e.market.as_deref() == Some(m))
                    && category.map_or(true, |c| e.category.as_deref() == Some(c))
            })
            .take(top_k)
            .cloned()
            .collect();
        if filtered.is_empty() {
            entries.iter().take(top_k).cloned().collect()
        } else {
            filtered
        }
    }
}
