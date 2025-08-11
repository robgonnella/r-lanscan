use std::{
    collections::HashMap,
    rc::Rc,
    sync::{Arc, mpsc::Sender},
};

use crate::ui::{
    components::{footer::InfoFooter, header::Header, popover::get_popover_area},
    events::types::Event,
    store::{
        Store,
        action::Action,
        state::{State, ViewID},
    },
};
use ratatui::{
    crossterm::event::{Event as CrossTermEvent, KeyCode},
    layout::{Constraint, Layout, Rect},
    style::{Style, palette::tailwind},
    text::Line,
    widgets::{Block, BorderType, Clear as ClearWidget, Padding, Paragraph, Widget, WidgetRef},
};

use super::{
    config::ConfigView,
    device::DeviceView,
    devices::DevicesView,
    traits::{CustomWidget, CustomWidgetContext, CustomWidgetRef, EventHandler, View},
    view_select::ViewSelect,
};

const DEFAULT_PADDING: Padding = Padding::horizontal(2);

pub struct MainView {
    store: Arc<Store>,
    sub_views: HashMap<ViewID, Box<dyn View>>,
    _tx: Sender<Event>,
}

impl MainView {
    pub fn new(store: Arc<Store>, tx: Sender<Event>) -> Self {
        let mut sub_views: HashMap<ViewID, Box<dyn View>> = HashMap::new();

        let config = Box::new(ConfigView::new(Arc::clone(&store)));
        let device = Box::new(DeviceView::new(Arc::clone(&store)));
        let devices = Box::new(DevicesView::new(Arc::clone(&store)));
        let view_select = Box::new(ViewSelect::new(
            vec![ViewID::Devices, ViewID::Config],
            2,
            Arc::clone(&store),
        ));

        sub_views.insert(config.id(), config);
        sub_views.insert(device.id(), device);
        sub_views.insert(devices.id(), devices);
        sub_views.insert(view_select.id(), view_select);

        Self {
            store,
            sub_views,
            _tx: tx,
        }
    }

    fn render_buffer_bg(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        let block = Block::new()
            .style(Style::new().bg(state.colors.buffer_bg))
            .padding(DEFAULT_PADDING);
        block.render(area, buf);
    }

    fn get_top_section_areas(&self, area: Rect) -> Rc<[Rect]> {
        Layout::horizontal([
            Constraint::Percentage(20),
            Constraint::Percentage(100),
            Constraint::Percentage(20),
        ])
        .split(area)
    }

    fn render_top(
        &self,
        sections: Rc<[Rect]>,
        buf: &mut ratatui::prelude::Buffer,
        message: Option<String>,
        ctx: &CustomWidgetContext,
    ) {
        let logo =
            Paragraph::new("\nr-lanterm").style(Style::new().fg(ctx.state.colors.border_color));
        let logo_block: Block<'_> = Block::bordered()
            .border_style(Style::new().fg(ctx.state.colors.border_color))
            .border_type(BorderType::Double)
            .padding(DEFAULT_PADDING);
        let logo_inner_area = logo_block.inner(sections[0]);

        logo_block.render(sections[0], buf);
        logo.render_ref(logo_inner_area, buf);

        if let Some(message) = message {
            let message_block = Block::default().padding(Padding::uniform(2));
            let message_inner_area = message_block.inner(sections[1]);
            let m = Header::new(format!("\n\n{message}"));
            message_block.render(sections[1], buf);
            m.render(message_inner_area, buf, ctx);
        }

        let current_view = Paragraph::new(format!("\n{} ▼", ctx.state.view_id))
            .style(Style::new().fg(ctx.state.colors.border_color));
        let current_view_block = Block::bordered()
            .border_style(Style::new().fg(ctx.state.colors.border_color))
            .border_type(BorderType::Double)
            .padding(DEFAULT_PADDING);
        let current_view_inner_area = current_view_block.inner(sections[2]);

        current_view_block.render(sections[2], buf);
        current_view.render_ref(current_view_inner_area, buf);
    }

    fn render_middle_view(
        &self,
        view: &dyn View,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        let block: Block<'_> = Block::bordered()
            .border_style(Style::new().fg(ctx.state.colors.border_color))
            .border_type(BorderType::Plain)
            .padding(DEFAULT_PADDING);
        let inner_area = block.inner(area);
        block.render(area, buf);
        view.render_ref(inner_area, buf, ctx);
    }

    fn render_view_select_popover(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        let view = self.sub_views.get(&ViewID::ViewSelect);

        if let Some(view_select) = view {
            view_select.render_ref(area, buf, ctx);
        }
    }

    fn render_error_popover(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        if state.error.is_some() {
            let msg = state.error.clone().unwrap();
            let block = Block::bordered()
                .border_type(BorderType::Double)
                .border_style(
                    Style::new()
                        .fg(tailwind::RED.c600)
                        .bg(state.colors.buffer_bg),
                )
                .padding(Padding::uniform(2))
                .style(Style::default().bg(state.colors.buffer_bg));
            let inner_area = block.inner(area);
            let [msg_area, exit_area] = Layout::vertical([
                Constraint::Percentage(100), // msg
                Constraint::Length(1),       // exit
            ])
            .areas(inner_area);

            let message = Line::from(format!("Error: {}", msg));
            let exit = Paragraph::new("Press enter to clear error").centered();
            ClearWidget.render(area, buf);
            block.render(area, buf);
            message.render(msg_area, buf);
            exit.render(exit_area, buf);
        }
    }

    fn render_footer(
        &self,
        legend: &str,
        override_legend: bool,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        if override_legend {
            let footer = InfoFooter::new(legend.to_string());
            footer.render(area, buf, ctx);
        } else {
            let mut info = String::from("(q) quit | (v) change view");

            if !legend.is_empty() {
                info = format!("{info} | {legend}");
            }

            let footer = InfoFooter::new(info);
            footer.render(area, buf, ctx);
        }
    }
}

impl View for MainView {
    fn id(&self) -> ViewID {
        ViewID::Main
    }
}

impl CustomWidgetRef for MainView {
    fn render_ref(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        // consists of 3 vertical rectangles (top, middle, bottom)
        let page_areas = Layout::vertical([
            Constraint::Length(5),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

        let view_id = ctx.state.view_id.clone();
        let view = self.sub_views.get(&view_id).unwrap();
        let legend = view.legend(&ctx.state);
        let override_legend = view.override_main_legend(&ctx.state);

        // render background for entire display
        self.render_buffer_bg(area, buf, &ctx.state);
        // logo & view select
        let top_section_areas = self.get_top_section_areas(page_areas[0]);
        let top_section_areas_clone = Rc::clone(&top_section_areas);
        self.render_top(top_section_areas, buf, ctx.state.message.clone(), ctx);
        // view
        self.render_middle_view(view.as_ref(), page_areas[1], buf, ctx);
        // legend for current view
        self.render_footer(legend, override_legend, page_areas[2], buf, ctx);

        // view selection
        if ctx.state.render_view_select {
            let mut select_area = top_section_areas_clone[2];
            select_area.height = (self.sub_views.len() * 3).try_into().unwrap();

            let select_block = Block::bordered()
                .border_style(Style::new().fg(ctx.state.colors.border_color))
                .border_type(BorderType::Double);

            let select_inner_area = select_block.inner(select_area);

            select_block.render(select_area, buf);

            ClearWidget.render(select_inner_area, buf);
            self.render_buffer_bg(select_inner_area, buf, &ctx.state);
            self.render_view_select_popover(select_inner_area, buf, ctx);
        }

        // popover when there are errors in the store
        // important to render this last so it properly layers on top
        self.render_error_popover(get_popover_area(area, 50, 40), buf, &ctx.state);
    }
}

impl EventHandler for MainView {
    fn process_event(&self, evt: &CrossTermEvent, ctx: &CustomWidgetContext) -> bool {
        if ctx.state.render_view_select {
            let select_view = self.sub_views.get(&ViewID::ViewSelect).unwrap();
            return select_view.process_event(evt, ctx);
        }

        if ctx.state.error.is_some() {
            if let CrossTermEvent::Key(key) = evt
                && key.code == KeyCode::Enter
            {
                self.store.dispatch(Action::SetError(None));
            }
            true
        } else {
            let view_id = ctx.state.view_id.clone();
            let view = self.sub_views.get(&view_id).unwrap();
            let mut handled = view.process_event(evt, ctx);

            if !handled && let CrossTermEvent::Key(key) = evt {
                match key.code {
                    KeyCode::Char('v') => {
                        if !ctx.state.render_view_select {
                            handled = true;
                            self.store.dispatch(Action::ToggleViewSelect);
                        }
                    }
                    KeyCode::Esc => {
                        if ctx.state.render_view_select {
                            handled = true;
                            self.store.dispatch(Action::ToggleViewSelect);
                        }
                    }
                    _ => {}
                }
            }

            handled
        }
    }
}

#[cfg(test)]
#[path = "./main_tests.rs"]
mod tests;
