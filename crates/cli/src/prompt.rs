use anyhow::Result;
use reedline::{DefaultPrompt, DefaultPromptSegment, Reedline, Signal};

pub struct Prompt {
    editor: Reedline,
}

impl Prompt {
    pub fn new() -> Self { Self { editor: Reedline::create() } }

    pub fn readline(&mut self) -> Result<Option<String>> {
        let prompt = DefaultPrompt {
            left_prompt:  DefaultPromptSegment::Basic("nxc> ".into()),
            right_prompt: DefaultPromptSegment::Empty,
        };
        match self.editor.read_line(&prompt) {
            Ok(Signal::Success(s)) => Ok(Some(s)),
            Ok(Signal::CtrlD | Signal::CtrlC) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }
}
