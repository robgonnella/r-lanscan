use color_eyre::eyre::Result;
use ratatui::{
    layout::{Constraint, Layout},
    widgets::{Paragraph, Widget},
};

use crate::ui::views::traits::{CustomWidgetContext, CustomWidgetRef};

pub struct SimplePopover {
    message: String,
    footer: Option<String>,
    message_centered: bool,
    footer_centered: bool,
}

impl SimplePopover {
    pub fn new<S: Into<String>>(message: S) -> Self {
        Self {
            message: message.into(),
            footer: None,
            message_centered: false,
            footer_centered: false,
        }
    }

    pub fn message_centered(mut self) -> Self {
        self.message_centered = true;
        self
    }

    pub fn footer<F: Into<String>>(mut self, f: F) -> Self {
        self.footer = Some(f.into());
        self
    }

    pub fn footer_centered(mut self) -> Self {
        self.footer_centered = true;
        self
    }
}

impl CustomWidgetRef for SimplePopover {
    fn render_ref(
        &self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        _ctx: &CustomWidgetContext,
    ) -> Result<()> {
        let mut msg = Paragraph::new(self.message.to_string());

        if self.message_centered {
            msg = msg.centered();
        }

        if let Some(footer_msg) = self.footer.as_ref() {
            let [msg_area, footer_area] = Layout::vertical([
                Constraint::Percentage(100), // msg
                Constraint::Length(1),       // exit
            ])
            .areas(area);

            let mut footer = Paragraph::new(footer_msg.to_string());

            if self.footer_centered {
                footer = footer.centered();
            }

            msg.render(msg_area, buf);
            footer.render(footer_area, buf);
        } else {
            msg.render(area, buf);
        }

        Ok(())
    }
}

#[cfg(test)]
#[path = "./simple_tests.rs"]
mod tests;
