use std::{collections::HashMap, rc::Rc, sync::Arc};

use crate::ui::{
    components::{footer::InfoFooter, header::Header},
    store::{
        action::Action,
        dispatcher::Dispatcher,
        state::{State, ViewID},
    },
};
use ratatui::{
    crossterm::event::{Event, KeyCode},
    layout::{Constraint, Layout, Rect},
    style::Style,
    widgets::{Block, BorderType, Clear, Padding, Paragraph, Widget, WidgetRef},
};

use super::{
    config::ConfigView, device::DeviceView, devices::DevicesView, view_select::ViewSelect,
    CustomWidget, EventHandler, View,
};

const DEFAULT_PADDING: Padding = Padding::horizontal(2);

pub struct MainView {
    dispatcher: Arc<Dispatcher>,
    sub_views: HashMap<ViewID, Box<dyn View>>,
}

impl MainView {
    pub fn new(dispatcher: Arc<Dispatcher>) -> Self {
        let mut sub_views: HashMap<ViewID, Box<dyn View>> = HashMap::new();

        let config = Box::new(ConfigView::new(Arc::clone(&dispatcher)));
        let device = Box::new(DeviceView::new(Arc::clone(&dispatcher)));
        let devices = Box::new(DevicesView::new(Arc::clone(&dispatcher)));
        let view_select = Box::new(ViewSelect::new(
            vec![ViewID::Devices, ViewID::Config],
            2,
            Arc::clone(&dispatcher),
        ));

        sub_views.insert(config.id(), config);
        sub_views.insert(device.id(), device);
        sub_views.insert(devices.id(), devices);
        sub_views.insert(view_select.id(), view_select);

        Self {
            dispatcher: Arc::clone(&dispatcher),
            sub_views,
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
        state: &State,
        message: Option<String>,
    ) {
        let logo = Paragraph::new("\nr-lanui").style(Style::new().fg(state.colors.border_color));
        let logo_block: Block<'_> = Block::bordered()
            .border_style(Style::new().fg(state.colors.border_color))
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
            m.render(message_inner_area, buf, state);
        }

        let current_view = Paragraph::new(format!("\n{} ▼", state.view_id.to_string()))
            .style(Style::new().fg(state.colors.border_color));
        let current_view_block = Block::bordered()
            .border_style(Style::new().fg(state.colors.border_color))
            .border_type(BorderType::Double)
            .padding(DEFAULT_PADDING);
        let current_view_inner_area = current_view_block.inner(sections[2]);

        current_view_block.render(sections[2], buf);
        current_view.render_ref(current_view_inner_area, buf);
    }

    fn render_middle_view(
        &self,
        view: &Box<dyn View>,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &State,
    ) {
        let block: Block<'_> = Block::bordered()
            .border_style(Style::new().fg(state.colors.border_color))
            .border_type(BorderType::Plain)
            .padding(DEFAULT_PADDING);
        let inner_area = block.inner(area);
        block.render(area, buf);
        view.render_ref(inner_area, buf);
    }

    fn render_view_select_popover(&self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let view = self.sub_views.get(&ViewID::ViewSelect);

        if let Some(view_select) = view {
            view_select.render_ref(area, buf);
        }
    }

    fn render_footer(
        &self,
        legend: &str,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &State,
    ) {
        let mut info = String::from("(q) quit | (v) change view");

        if legend.len() > 0 {
            info = format!("{info} | {legend}");
        }

        let footer = InfoFooter::new(info);
        footer.render(area, buf, state);
    }
}

impl View for MainView {
    fn id(&self) -> ViewID {
        ViewID::Main
    }
}

impl WidgetRef for MainView {
    fn render_ref(&self, area: Rect, buf: &mut ratatui::prelude::Buffer) {
        let state = self.dispatcher.get_state();

        // consists of 3 vertical rectangles (top, middle, bottom)
        let page_areas = Layout::vertical([
            Constraint::Length(5),
            Constraint::Min(10),
            Constraint::Length(3),
        ])
        .split(area);

        let view_id = self.dispatcher.get_state().view_id;
        let view = self.sub_views.get(&view_id).unwrap();
        let legend = view.legend();

        // render background for entire display
        self.render_buffer_bg(area, buf, &state);
        // logo & view select
        let top_section_areas = self.get_top_section_areas(page_areas[0]);
        let top_section_areas_clone = Rc::clone(&top_section_areas);
        self.render_top(top_section_areas, buf, &state, state.message.clone());
        // view
        self.render_middle_view(view, page_areas[1], buf, &state);
        // legend for current view
        self.render_footer(legend, page_areas[2], buf, &state);

        // view selection
        if state.render_view_select {
            let mut select_area = top_section_areas_clone[2];
            select_area.height = (self.sub_views.len() * 3).try_into().unwrap();

            let select_block = Block::bordered()
                .border_style(Style::new().fg(state.colors.border_color))
                .border_type(BorderType::Double);

            let select_inner_area = select_block.inner(select_area);

            select_block.render(select_area, buf);

            Clear.render(select_inner_area, buf);
            self.render_buffer_bg(select_inner_area, buf, &state);
            self.render_view_select_popover(select_inner_area, buf);
        }
    }
}

impl EventHandler for MainView {
    fn process_event(&mut self, evt: &Event, state: &State) -> bool {
        if state.render_view_select {
            let select_view = self.sub_views.get_mut(&ViewID::ViewSelect).unwrap();
            return select_view.process_event(evt, state);
        }

        let view_id = state.view_id.clone();
        let view = self.sub_views.get_mut(&view_id).unwrap();
        let mut handled = view.process_event(evt, state);

        if !handled {
            match evt {
                Event::Key(key) => match key.code {
                    KeyCode::Char('v') => {
                        if !state.render_view_select {
                            handled = true;
                            self.dispatcher.dispatch(Action::ToggleViewSelect);
                        }
                    }
                    KeyCode::Esc => {
                        if state.render_view_select {
                            handled = true;
                            self.dispatcher.dispatch(Action::ToggleViewSelect);
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }

        handled
    }
}
