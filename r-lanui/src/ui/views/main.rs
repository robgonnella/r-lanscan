use std::{collections::HashMap, sync::Arc};

use crate::ui::{
    components::{footer::InfoFooter, header::Header},
    store::{
        action::Action,
        dispatcher::Dispatcher,
        state::{State, ViewID},
    },
};
use ratatui::{
    crossterm::event::{Event, MouseEventKind},
    layout::{Constraint, Layout, Position, Rect},
    style::Style,
    widgets::{Block, BorderType, Borders, Padding, Paragraph, Widget, WidgetRef},
};

use super::{
    config::ConfigView, device::DeviceView, devices::DevicesView, CustomWidget, EventHandler, View,
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

        sub_views.insert(config.id(), config);
        sub_views.insert(device.id(), device);
        sub_views.insert(devices.id(), devices);

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

    fn render_top(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &State,
        message: Option<String>,
    ) {
        let sections = Layout::horizontal([
            Constraint::Percentage(20),
            Constraint::Percentage(100),
            Constraint::Percentage(20),
        ])
        .split(area);

        let logo =
            Paragraph::new("\nr-lanui").style(Style::new().fg(state.colors.border_focused_color));
        let logo_block: Block<'_> = Block::bordered()
            .border_style(Style::new().fg(state.colors.border_focused_color))
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

        let search = Paragraph::new("\nSearch").style(Style::new().fg(state.colors.placeholder));
        let search_block = Block::bordered()
            .border_style(Style::new().fg(state.colors.border_focused_color))
            .padding(DEFAULT_PADDING);
        let search_inner_area = search_block.inner(sections[2]);

        search_block.render(sections[2], buf);
        search.render_ref(search_inner_area, buf);
    }

    fn render_search(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        let block: Block<'_> =
            Block::bordered().border_style(Style::new().fg(state.colors.border_color));
        block.render(area, buf);
    }

    fn render_devices(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        let mut border_color = state.colors.border_color;
        let mut border_type = BorderType::Plain;

        if state.focused == ViewID::Devices {
            border_color = state.colors.border_focused_color;
            border_type = BorderType::Double
        }

        let block: Block<'_> = Block::bordered()
            .border_style(Style::new().fg(border_color))
            .border_type(border_type)
            .padding(DEFAULT_PADDING);
        let devices = self.sub_views.get(&ViewID::Devices).unwrap();
        let inner_area = block.inner(area);

        block.render(area, buf);
        devices.render_ref(inner_area, buf);
    }

    fn render_selected_host(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        let mut border_color = state.colors.border_color;

        if state.focused == ViewID::Device {
            border_color = state.colors.border_focused_color;
        }

        let block: Block<'_> = Block::bordered()
            .border_style(Style::new().fg(border_color))
            .padding(DEFAULT_PADDING);
        let device = self.sub_views.get(&ViewID::Device).unwrap();
        let inner_area = block.inner(area);

        block.render(area, buf);
        device.render_ref(inner_area, buf);
    }

    fn render_host_view_port(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        let block = Block::new()
            .borders(Borders::all())
            .border_style(Style::new().fg(state.colors.border_color))
            .padding(DEFAULT_PADDING);

        let inner_area = block.inner(area);

        let view_port_vects =
            Layout::vertical([Constraint::Length(1), Constraint::Min(5)]).split(inner_area);

        let label_vects = Layout::horizontal([Constraint::Length(20)]).split(view_port_vects[0]);

        let header = Header::new(String::from("Viewport"));

        block.render(area, buf);
        header.render(label_vects[0], buf, state);
    }

    fn render_config(&self, area: Rect, buf: &mut ratatui::prelude::Buffer, state: &State) {
        let mut border_color = state.colors.border_color;

        if state.focused == ViewID::Config {
            border_color = state.colors.border_focused_color;
        }

        let block: Block<'_> = Block::bordered()
            .border_style(Style::new().fg(border_color))
            .padding(DEFAULT_PADDING);
        let config_view = self.sub_views.get(&ViewID::Config).unwrap();
        let inner_area = block.inner(area);

        block.render(area, buf);
        config_view.render_ref(inner_area, buf);
    }

    fn render_footer(
        &self,
        legend: &str,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        state: &State,
    ) {
        let mut info = String::from("(Esc) quit");

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

        // split middle into 3 columns (left, middle, right)
        let middle_areas = Layout::horizontal([
            Constraint::Percentage(20),
            Constraint::Percentage(100),
            Constraint::Percentage(20),
        ])
        .split(page_areas[1]);

        // split middle column of middle area into 2 vertical regions (top, bottom)
        let middle_column_sections =
            Layout::vertical([Constraint::Percentage(34), Constraint::Percentage(66)])
                .split(middle_areas[1]);

        let mut layout: HashMap<ViewID, Rect> = HashMap::new();
        layout.insert(ViewID::Config, middle_areas[2]);
        layout.insert(ViewID::Device, middle_column_sections[0]);
        layout.insert(ViewID::Devices, middle_areas[0]);

        self.dispatcher.dispatch(Action::UpdateLayout(Some(layout)));

        let focused_id = self.dispatcher.get_state().focused;
        let view = self.sub_views.get(&focused_id).unwrap();
        let legend = view.legend();

        self.render_buffer_bg(area, buf, &state);
        self.render_top(page_areas[0], buf, &state, state.message.clone());
        self.render_search(page_areas[2], buf, &state);
        self.render_devices(middle_areas[0], buf, &state);
        self.render_selected_host(middle_column_sections[0], buf, &state);
        self.render_host_view_port(middle_column_sections[1], buf, &state);
        self.render_config(middle_areas[2], buf, &state);
        self.render_footer(legend, page_areas[2], buf, &state);
    }
}

impl EventHandler for MainView {
    fn process_event(&mut self, evt: &Event) -> bool {
        let focused_id = self.dispatcher.get_state().focused;
        let view = self.sub_views.get_mut(&focused_id).unwrap();
        let handled = view.process_event(evt);

        match evt {
            Event::FocusGained => {}
            Event::FocusLost => {}
            Event::Mouse(m) => match m.kind {
                MouseEventKind::Up(_b) => {
                    self.dispatcher.dispatch(Action::Click(Position {
                        x: m.column,
                        y: m.row,
                    }));
                }
                _ => {}
            },
            Event::Paste(_s) => {}
            Event::Resize(_x, _y) => {}
            Event::Key(_key) => {}
        }

        handled
    }
}
