use reco_core::chat::{ChatMessage, ChatRole};
use reco_core::infer::{InferEngine, PickedEngine};
#[cfg(test)]
use reco_core::infer::EchoEngine;
use reco_core::store::ChatStore;
use reco_core::Recommendation;

pub struct PruebaSession {
    pub conversation_id: String,
    pub repo_id: String,
    pub filename: String,
    pub engine_label: String,
    pub messages: Vec<ChatMessage>,
    pub input: String,
    pub status: String,
    pub offset: usize,
    pub show_help: bool,
    engine: Box<dyn InferEngine>,
}

impl PruebaSession {
    pub fn open(
        store: &ChatStore,
        rec: &Recommendation,
        mut picked: PickedEngine,
    ) -> Result<Self, String> {
        let conv = store
            .open_or_create(&rec.repo_id, &rec.filename)
            .map_err(|err| err.to_string())?;
        let messages = store.messages(&conv.id).map_err(|err| err.to_string())?;
        if messages.is_empty() {
            let welcome = picked
                .engine
                .generate(&[])
                .unwrap_or_else(|_| "Prueba lista.".into());
            store
                .append(&conv.id, ChatRole::Assistant, &welcome)
                .map_err(|err| err.to_string())?;
        }
        let messages = store.messages(&conv.id).map_err(|err| err.to_string())?;
        Ok(Self {
            conversation_id: conv.id,
            repo_id: rec.repo_id.clone(),
            filename: rec.filename.clone(),
            engine_label: picked.label,
            messages,
            input: String::new(),
            status: "enter enviar  ·  ^n chat nuevo  ·  ? ayuda  ·  esc salir".into(),
            offset: 0,
            show_help: false,
            engine: picked.engine,
        })
    }

    #[cfg(test)]
    pub fn echo(store: &ChatStore, rec: &Recommendation) -> Result<Self, String> {
        Self::open(
            store,
            rec,
            PickedEngine {
                engine: Box::new(EchoEngine::new(rec.repo_id.clone())),
                label: "echo".into(),
                hint: None,
            },
        )
    }

    pub fn type_char(&mut self, ch: char) {
        if !ch.is_control() {
            self.input.push(ch);
        }
    }

    pub fn backspace(&mut self) {
        self.input.pop();
    }

    pub fn submit(&mut self, store: &ChatStore) -> Result<(), String> {
        let text = self.input.trim().to_string();
        if text.is_empty() {
            return Ok(());
        }
        self.input.clear();
        store
            .append(&self.conversation_id, ChatRole::User, &text)
            .map_err(|err| err.to_string())?;
        self.messages.push(ChatMessage {
            role: ChatRole::User,
            content: text,
        });
        self.status = "pensando…".into();
        let reply = self
            .engine
            .generate(&self.messages)
            .map_err(|err| err.to_string())?;
        store
            .append(&self.conversation_id, ChatRole::Assistant, &reply)
            .map_err(|err| err.to_string())?;
        self.messages.push(ChatMessage {
            role: ChatRole::Assistant,
            content: reply,
        });
        self.status = "enter enviar  ·  ^n chat nuevo  ·  ? ayuda  ·  esc salir".into();
        self.offset = 0;
        Ok(())
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn page_up(&mut self) {
        self.offset = self.offset.saturating_add(8);
    }

    pub fn page_down(&mut self) {
        self.offset = self.offset.saturating_sub(8);
    }

    pub fn new_chat(&mut self, store: &ChatStore) -> Result<(), String> {
        let conv = store
            .new_conversation(&self.repo_id, &self.filename)
            .map_err(|err| err.to_string())?;
        let welcome = self
            .engine
            .generate(&[])
            .unwrap_or_else(|_| "Prueba lista.".into());
        store
            .append(&conv.id, ChatRole::Assistant, &welcome)
            .map_err(|err| err.to_string())?;
        self.conversation_id = conv.id;
        self.messages = store
            .messages(&self.conversation_id)
            .map_err(|err| err.to_string())?;
        self.offset = 0;
        self.status = "chat nuevo".into();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use reco_core::{format_gib, GgufQuant, ModelParams, Scores};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn rec() -> Recommendation {
        Recommendation {
            repo_id: "Qwen/Qwen2.5-7B-Instruct-GGUF".into(),
            filename: "q4.gguf".into(),
            quant: GgufQuant::Q4Km,
            size_bytes: 4,
            size_estimated: true,
            params: Some(ModelParams {
                total_billions: 7.0,
                active_billions: None,
            }),
            downloads: 1,
            scores: Scores {
                compatibility: 90.0,
                speed: 80.0,
                quality: 70.0,
                popularity: 60.0,
            },
            total: 80.0,
            why: String::new(),
        }
    }

    #[test]
    fn submit_persists_both_turns() {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let dir = std::env::temp_dir().join(format!("reco-prueba-{stamp}"));
        let store = ChatStore::open(&dir.join("reco.db")).unwrap();
        let model = rec();
        let mut session = PruebaSession::echo(&store, &model).unwrap();
        assert!(!session.messages.is_empty());
        session.input = "hola desde el test".into();
        session.submit(&store).unwrap();
        let msgs = store.messages(&session.conversation_id).unwrap();
        assert!(msgs
            .iter()
            .any(|m| m.content.contains("hola desde el test")));
        assert!(msgs
            .iter()
            .any(|m| m.role == ChatRole::Assistant && m.content.contains("demo")));
        let _ = format_gib(1);
        let _ = std::fs::remove_dir_all(dir);
    }
}
