use nxc_provider::Message;

pub struct History {
    pub messages:  Vec<Message>,
    context_limit: u32,
}

impl History {
    pub fn new(context_limit: u32) -> Self { Self { messages: vec![], context_limit } }

    pub fn push(&mut self, msg: Message) { self.messages.push(msg); }

    pub fn truncate_to_limit(&mut self) {
        let system = self.messages.first().filter(|m| m.role == "system").cloned();
        let limit  = self.context_limit as usize * 4;
        while self.char_count() > limit && self.messages.len() > 2 {
            let start = if system.is_some() { 1 } else { 0 };
            self.messages.remove(start);
        }
    }

    fn char_count(&self) -> usize {
        self.messages.iter().map(|m| {
            m.content.as_ref().map(|c| c.to_string().len()).unwrap_or(0)
        }).sum()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nxc_provider::Message;

    #[test]
    fn truncates_when_over_limit() {
        let mut h = History::new(10);
        h.push(Message::system("sys"));
        for i in 0..20 { h.push(Message::user(format!("msg {i}"))); }
        h.truncate_to_limit();
        assert_eq!(h.messages[0].role, "system");
        assert!(h.messages.len() < 22);
    }
}
