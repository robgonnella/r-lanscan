use std::{cell::RefCell, rc::Rc};

use color_eyre::eyre::Result;
use ratatui::{
    layout::{Constraint, Layout},
    widgets::{Paragraph, Widget},
};

use crate::ui::{
    components::{
        header::Header,
        input::{Input, InputState},
    },
    views::traits::{
        CustomStatefulWidget, CustomWidget, CustomWidgetContext,
        CustomWidgetRef,
    },
};

pub struct BrowsePopover {
    browser_state: Rc<RefCell<InputState>>,
    port_state: Rc<RefCell<InputState>>,
}

impl BrowsePopover {
    pub fn new(
        browser_state: Rc<RefCell<InputState>>,
        port_state: Rc<RefCell<InputState>>,
    ) -> Self {
        Self {
            browser_state,
            port_state,
        }
    }
}

impl CustomWidgetRef for BrowsePopover {
    fn render_ref(
        &self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) -> Result<()> {
        let [header_area, _, browser_area, port_area, message_area] =
            Layout::vertical([
                Constraint::Length(1),       // header
                Constraint::Length(1),       // spacer
                Constraint::Percentage(50),  // browser choice
                Constraint::Percentage(100), // port select
                Constraint::Length(1),       // enter to submit message
            ])
            .areas(area);

        let header = Header::new("Enter port to browse".to_string());
        let browser_input = Input::new("Browser Select <->");
        let port_input = Input::new("Port");
        let message =
            Paragraph::new("Press enter to open browser or esc to cancel")
                .centered();

        header.render(header_area, buf, ctx);

        browser_input.render(
            browser_area,
            buf,
            &mut self.browser_state.borrow_mut(),
            ctx,
        );
        port_input.render(
            port_area,
            buf,
            &mut self.port_state.borrow_mut(),
            ctx,
        );
        message.render(message_area, buf);

        Ok(())
    }
}

#[cfg(test)]
#[path = "./browse_tests.rs"]
mod tests;
