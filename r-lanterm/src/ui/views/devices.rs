use itertools::Itertools;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind},
    layout::{Constraint, Layout, Rect},
};
use std::{cell::RefCell, sync::Arc};

use crate::ui::{
    components::table::{self, Table},
    store::{
        action::Action,
        state::{State, ViewID},
        store::Store,
    },
};

use super::traits::{CustomWidgetContext, CustomWidgetRef, EventHandler, View};

pub struct DevicesView {
    store: Arc<Store>,
    table: RefCell<Table>,
}

impl DevicesView {
    pub fn new(store: Arc<Store>) -> Self {
        let state = store.get_state();

        let items = state
            .devices
            .iter()
            .map(|d| {
                vec![
                    d.ip.clone(),
                    d.hostname.clone(),
                    d.vendor.clone(),
                    d.mac.clone(),
                    d.open_ports
                        .iter()
                        .sorted_by_key(|p| p.id)
                        .map(|p| p.id.to_string())
                        .join(", "),
                ]
            })
            .collect_vec();

        let mut height = table::DEFAULT_ITEM_HEIGHT;

        if state.devices.len() > 0 {
            height = (state.devices.len() - 1) * table::DEFAULT_ITEM_HEIGHT;
        }

        Self {
            store,
            table: RefCell::new(Table::new(
                items,
                Some(vec![
                    "IP".to_string(),
                    "HOSTNAME".to_string(),
                    "VENDOR".to_string(),
                    "MAC".to_string(),
                    "OPEN PORTS".to_string(),
                ]),
                vec![20, 20, 20, 17, 30],
                height,
            )),
        }
    }

    fn next(&self) {
        self.table.borrow_mut().next();
    }

    fn previous(&self) {
        self.table.borrow_mut().previous();
    }

    fn set_store_selected(&self, i: usize, state: &State) {
        if state.devices.len() > 0 && i < state.devices.len() {
            let mac = state.devices[i].mac.clone();
            self.store.dispatch(Action::UpdateSelectedDevice(mac));
        }
    }

    fn handle_device_selection(&self, state: &State) {
        if state.selected_device.is_some() {
            self.store.dispatch(Action::UpdateView(ViewID::Device));
        }
    }

    fn render_table(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        let items = ctx
            .state
            .devices
            .iter()
            .map(|d| {
                vec![
                    if d.is_current_host {
                        format!("{} [YOU]", d.ip.clone())
                    } else {
                        d.ip.clone()
                    },
                    d.hostname.clone(),
                    d.vendor.clone(),
                    d.mac.clone(),
                    d.open_ports
                        .iter()
                        .sorted_by_key(|p| p.id)
                        .map(|p| p.id.to_string())
                        .join(", "),
                ]
            })
            .collect_vec();
        let selected = self.table.borrow_mut().update_items(items);
        if let Some(selected) = selected {
            self.set_store_selected(selected, &ctx.state);
        }
        self.table.borrow().render_ref(area, buf, ctx);
    }
}

impl View for DevicesView {
    fn id(&self) -> ViewID {
        ViewID::Devices
    }

    fn legend(&self, _state: &State) -> &str {
        "(enter) view device details"
    }
}

impl CustomWidgetRef for DevicesView {
    fn render_ref(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        if let Some(selected_idx) = self.table.borrow().selected() {
            self.set_store_selected(selected_idx, &ctx.state);
        }

        let view_rects = Layout::vertical([Constraint::Length(1), Constraint::Min(5)]).split(area);

        self.render_table(view_rects[1], buf, ctx);
    }
}

impl EventHandler for DevicesView {
    fn process_event(&self, evt: &Event, ctx: &CustomWidgetContext) -> bool {
        if ctx.state.render_view_select {
            return false;
        }

        let mut handled = false;

        match evt {
            Event::FocusGained => {}
            Event::FocusLost => {}
            Event::Mouse(_m) => {}
            Event::Paste(_s) => {}
            Event::Resize(_x, _y) => {}
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            self.next();
                            handled = true;
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            self.previous();
                            handled = true;
                        }
                        KeyCode::Enter => {
                            self.handle_device_selection(&ctx.state);
                            handled = true;
                        }
                        _ => {}
                    }
                }
            }
        }

        handled
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
        collections::{HashMap, HashSet},
        fs,
        sync::Mutex,
    };

    use crate::config::{Config, ConfigManager};

    use super::*;

    fn setup() -> (DevicesView, Arc<Store>, String) {
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
        (DevicesView::new(Arc::clone(&store)), store, tmp_path)
    }

    fn tear_down(conf_path: String) {
        fs::remove_file(conf_path).unwrap();
    }

    #[test]
    fn test_devices_view() {
        let (devs_view, store, conf_path) = setup();
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

                devs_view.render_ref(frame.area(), frame.buffer_mut(), &ctx);
            })
            .unwrap();

        assert_snapshot!(terminal.backend());
        tear_down(conf_path);
    }
}
