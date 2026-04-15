use common::types::Model;

/// Classifica il prompt e sceglie il modello più adeguato al minimo costo.
/// Nessuna chiamata API — logica basata su keyword, lunghezza e segnali strutturali.
///
/// Tier 1 — Haiku  : comandi di sistema, azioni semplici, domande rapide
/// Tier 2 — Sonnet : scrittura, codice, analisi, blog, spiegazioni
/// Tier 3 — Opus   : ragionamento approfondito, piani multi-step, documenti lunghi
pub struct TaskRouter;

impl TaskRouter {
    pub fn new() -> Self { Self }

    pub async fn classify(&self, prompt: &str) -> Model {
        let p = prompt.to_lowercase();

        // ── Tier 3: Opus ───────────────────────────────────────────────────
        // Ragionamento strategico, pianificazione complessa, analisi estesa.
        let opus_keywords = [
            "progetta", "design", "architettura", "architecture",
            "piano strategico", "strategic plan", "roadmap",
            "confronta in dettaglio", "compare in detail",
            "analisi approfondita", "deep analysis", "analisi completa",
            "ragiona su", "reason about", "valuta pro e contro",
            "ottimizza il sistema", "refactor completo",
        ];
        for kw in &opus_keywords {
            if p.contains(kw) {
                return Model::Opus;
            }
        }
        // Prompt molto lunghi con terminologia complessa → Opus
        if prompt.len() > 800 {
            return Model::Opus;
        }

        // ── Tier 2: Sonnet ─────────────────────────────────────────────────
        // Scrittura, codice, analisi media, blog, spiegazioni dettagliate.
        let sonnet_keywords = [
            // scrittura
            "scrivi", "write", "redigi", "draft", "componi", "compose",
            "articolo", "article", "blog", "post", "newsletter",
            "revisiona", "review", "correggi", "proofreading",
            // codice
            "codice", "code", "funzione", "function", "script",
            "implementa", "implement", "programma", "program",
            "genera", "generate", "crea un", "create a",
            // analisi/spiegazione
            "analizza", "analyze", "analyse", "spiega", "explain",
            "riassumi", "summarize", "traduci", "translate",
            "elenca i passaggi", "step by step", "passo per passo",
            // contenuto lungo
            "in dettaglio", "in detail", "dettagliatamente",
        ];
        for kw in &sonnet_keywords {
            if p.contains(kw) {
                return Model::Sonnet;
            }
        }
        // Prompt medi → Sonnet
        if prompt.len() > 200 {
            return Model::Sonnet;
        }

        // ── Tier 1: Haiku ──────────────────────────────────────────────────
        // Comandi di sistema, domande rapide, azioni discrete, status.
        Model::Haiku
    }
}
