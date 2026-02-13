use std::borrow::Cow;
use reedline::{Color, DefaultPrompt, Prompt, PromptEditMode, PromptHistorySearch};
use shared::State;

pub struct NihilPrompt {
    pub state: State,
    inner: DefaultPrompt,
}

impl NihilPrompt {
    pub fn new(state: State) -> Self {
        Self {
            state,
            inner: DefaultPrompt::default(),
        }
    }
}

impl Prompt for NihilPrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Owned("nihil_elegans".into())
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        self.inner.render_prompt_right()
    }

    fn render_prompt_indicator(&self, prompt_mode: PromptEditMode) -> Cow<'_, str> {
        self.inner.render_prompt_indicator(prompt_mode)
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        self.inner.render_prompt_multiline_indicator()
    }

    fn render_prompt_history_search_indicator(&self, history_search: PromptHistorySearch) -> Cow<'_, str> {
        self.inner.render_prompt_history_search_indicator(history_search)
    }

    fn get_prompt_color(&self) -> Color {
        Color::DarkRed
    }

    fn get_indicator_color(&self) -> Color {
        Color::Cyan
    }

    fn get_prompt_right_color(&self) -> Color {
        Color::Red
    }
}