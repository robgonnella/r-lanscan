use std::{
    collections::HashMap,
    rc::Rc,
    sync::{mpsc::Sender, Arc},
};

use crate::ui::{
    components::{footer::InfoFooter, header::Header, popover::get_popover_area},
    events::types::Event,
    store::{
        action::Action,
        state::{State, ViewID},
        store::Store,
    },
};
use ratatui::{
    crossterm::event::{Event as CrossTermEvent, KeyCode},
    layout::{Constraint, Layout, Rect},
    style::{palette::tailwind, Style},
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
            Paragraph::new("\nr-lanui").style(Style::new().fg(ctx.state.colors.border_color));
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

        let current_view = Paragraph::new(format!("\n{} ▼", ctx.state.view_id.to_string()))
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
        view: &Box<dyn View>,
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

            if legend.len() > 0 {
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
        self.render_middle_view(view, page_areas[1], buf, ctx);
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
            match evt {
                CrossTermEvent::Key(key) => match key.code {
                    KeyCode::Enter => {
                        self.store.dispatch(Action::SetError(None));
                    }
                    _ => {}
                },
                _ => {}
            }
            true
        } else {
            let view_id = ctx.state.view_id.clone();
            let view = self.sub_views.get(&view_id).unwrap();
            let mut handled = view.process_event(evt, ctx);

            if !handled {
                match evt {
                    CrossTermEvent::Key(key) => match key.code {
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
                    },
                    _ => {}
                }
            }

            handled
        }
    }
}

#[cfg(test)]
mod tests {
    use insta::assert_snapshot;
    use nanoid::nanoid;
    use pnet::util::MacAddr;
    use r_lanlib::scanners::{DeviceWithPorts, Port};
    use ratatui::{backend::TestBackend, Terminal};
    use std::{
        collections::HashSet,
        fs,
        sync::{mpsc, Mutex},
    };

    use crate::config::{Config, ConfigManager};

    use super::*;

    fn setup() -> (MainView, Arc<Store>, String) {
        fs::create_dir_all("generated").unwrap();
        let tmp_path = format!("generated/{}.yml", nanoid!());
        let conf_manager = Arc::new(Mutex::new(ConfigManager::new(tmp_path.as_str())));
        let store = Arc::new(Store::new(conf_manager));
        let config = Config {
            id: "default".to_string(),
            cidr: "192.168.1.1/24".to_string(),
            default_ssh_identity: "id_rsa".to_string(),
            default_ssh_port: "22".to_string(),
            default_ssh_user: "user".to_string(),
            device_configs: HashMap::new(),
            ports: vec![],
            theme: "Blue".to_string(),
        };
        store.dispatch(Action::CreateAndSetConfig(config));

        let mut open_ports: HashSet<Port> = HashSet::new();
        open_ports.insert(Port {
            id: 80,
            service: "http".to_string(),
        });

        let device_1 = DeviceWithPorts {
            hostname: "hostname".to_string(),
            ip: "10.10.10.1".to_string(),
            is_current_host: false,
            mac: MacAddr::default().to_string(),
            open_ports: open_ports.clone(),
            vendor: "mac".to_string(),
        };

        let device_2 = DeviceWithPorts {
            hostname: "dev2_hostname".to_string(),
            ip: "10.10.10.2".to_string(),
            is_current_host: true,
            mac: "ff:ff:ff:ff:ff:ff".to_string(),
            open_ports,
            vendor: "linux".to_string(),
        };

        store.dispatch(Action::AddDevice(device_1.clone()));
        store.dispatch(Action::AddDevice(device_2.clone()));
        let (tx, _rx) = mpsc::channel();
        (MainView::new(Arc::clone(&store), tx), store, tmp_path)
    }

    fn tear_down(conf_path: String) {
        fs::remove_file(conf_path).unwrap();
    }

    #[test]
    fn test_main_view() {
        let (main_view, store, conf_path) = setup();
        let mut terminal = Terminal::new(TestBackend::new(80, 15)).unwrap();
        let state = store.get_state();
        let channel = std::sync::mpsc::channel();

        terminal
            .draw(|frame| {
                let ctx = CustomWidgetContext {
                    state,
                    app_area: frame.area(),
                    events: channel.0,
                };

                main_view.render_ref(frame.area(), frame.buffer_mut(), &ctx);
            })
            .unwrap();

        assert_snapshot!(terminal.backend());
        tear_down(conf_path);
    }
}
