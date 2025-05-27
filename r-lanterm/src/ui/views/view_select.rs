use std::{cell::RefCell, sync::Arc};

use itertools::Itertools;
use ratatui::crossterm::event::{Event, KeyCode, KeyEventKind};

use crate::ui::{
    components::table::{self, Table},
    store::{action::Action, state::ViewID, store::Store},
};

use super::traits::{CustomWidgetContext, CustomWidgetRef, EventHandler, View};

pub struct ViewSelect {
    store: Arc<Store>,
    view_ids: Vec<ViewID>,
    table: RefCell<Table>,
}

impl ViewSelect {
    pub fn new(view_ids: Vec<ViewID>, padding: usize, store: Arc<Store>) -> Self {
        let mut spacer = String::from("");

        if padding > 0 {
            for _ in 0..padding {
                spacer += " ";
            }
        }

        let table_items = view_ids
            .clone()
            .iter()
            .map(|v| vec![format!("{}{}", spacer, v.to_string())])
            .collect_vec();

        let mut table_select = Table::new(
            table_items,
            None,
            vec![15; view_ids.len()],
            table::DEFAULT_ITEM_HEIGHT,
        );

        table_select.next();

        Self {
            store,
            view_ids,
            table: RefCell::new(table_select),
        }
    }

    fn next(&self) {
        self.table.borrow_mut().next();
    }

    fn previous(&self) {
        self.table.borrow_mut().previous();
    }

    fn handle_selected(&self) {
        let i = self.table.borrow().selected();
        if let Some(selected) = i {
            let id = self.view_ids[selected].clone();
            self.store.dispatch(Action::UpdateView(id));
            self.store.dispatch(Action::ToggleViewSelect);
        }
    }
}

impl View for ViewSelect {
    fn id(&self) -> ViewID {
        ViewID::ViewSelect
    }
}

impl EventHandler for ViewSelect {
    fn process_event(&self, evt: &Event, ctx: &CustomWidgetContext) -> bool {
        if !ctx.state.render_view_select {
            return false;
        }

        let mut handled = false;

        match evt {
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
                        KeyCode::Esc => {
                            if ctx.state.render_view_select {
                                self.store.dispatch(Action::ToggleViewSelect);
                                handled = true;
                            }
                        }
                        KeyCode::Enter => {
                            self.handle_selected();
                            handled = true;
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }

        handled
    }
}

impl CustomWidgetRef for ViewSelect {
    fn render_ref(
        &self,
        area: ratatui::prelude::Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) {
        self.table.borrow().render_ref(area, buf, ctx);
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

    fn setup() -> (ViewSelect, Arc<Store>, String) {
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
        let view_ids = vec![ViewID::Devices, ViewID::Config];
        (
            ViewSelect::new(view_ids, 2, Arc::clone(&store)),
            store,
            tmp_path,
        )
    }

    fn tear_down(conf_path: String) {
        fs::remove_file(conf_path).unwrap();
    }

    #[test]
    fn test_view_select_view() {
        let (view_select, store, conf_path) = setup();
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

                view_select.render_ref(frame.area(), frame.buffer_mut(), &ctx);
            })
            .unwrap();

        assert_snapshot!(terminal.backend());
        tear_down(conf_path);
    }
}
