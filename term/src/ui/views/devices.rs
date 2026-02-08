//! Devices list view showing all discovered network devices in a table.

use color_eyre::eyre::Result;
use itertools::Itertools;
use r_lanlib::scanners::Device;
use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEventKind, MouseEventKind},
    layout::{Constraint, Layout, Rect},
};
use std::{cell::RefCell, sync::Arc};

use crate::{
    config::DeviceConfig,
    ui::{
        components::table::{DEFAULT_ITEM_HEIGHT, Table},
        store::{Dispatcher, state::State},
        views::{device::DeviceView, traits::CustomEventContext},
    },
};

use super::traits::{CustomWidgetContext, CustomWidgetRef, EventHandler, View};

/// Main view showing all discovered devices in a selectable table.
pub struct DevicesView {
    dispatcher: Arc<dyn Dispatcher>,
    selected_device: RefCell<Option<Device>>,
    device_view: RefCell<Option<Box<dyn View>>>,
    table: RefCell<Table>,
}

impl DevicesView {
    /// Creates a new devices view with the given dispatcher.
    pub fn new(dispatcher: Arc<dyn Dispatcher>) -> Self {
        Self {
            dispatcher,
            selected_device: RefCell::new(None),
            device_view: RefCell::new(None),
            table: RefCell::new(Table::new(
                Vec::new(),
                Some(vec![
                    "IP".to_string(),
                    "HOSTNAME".to_string(),
                    "VENDOR".to_string(),
                    "MAC".to_string(),
                    "OPEN PORTS".to_string(),
                ]),
                vec![25, 30, 30, 25, 20],
                DEFAULT_ITEM_HEIGHT,
            )),
        }
    }

    fn next(&self) {
        self.table.borrow_mut().next();
    }

    fn previous(&self) {
        self.table.borrow_mut().previous();
    }

    fn set_selected(&self, i: usize, state: &State) {
        if !state.sorted_device_list.is_empty()
            && i < state.sorted_device_list.len()
        {
            let device = state.sorted_device_list[i].clone();
            let device_view = self.get_device_view(&device, state);
            *self.selected_device.borrow_mut() =
                Some(state.sorted_device_list[i].clone());
            *self.device_view.borrow_mut() = Some(device_view);
        }
    }

    fn render_table(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) -> Result<()> {
        let devices = ctx.state.sorted_device_list.clone();

        let items = devices
            .iter()
            .map(|d| {
                vec![
                    if d.is_current_host {
                        format!("{} [YOU]", d.ip)
                    } else {
                        d.ip.to_string()
                    },
                    d.hostname.clone(),
                    d.vendor.clone(),
                    d.mac.to_string(),
                    d.open_ports
                        .to_sorted_vec()
                        .iter()
                        .map(|p| p.to_string())
                        .join(", "),
                ]
            })
            .collect_vec();

        self.table.borrow_mut().update_items(items);

        self.table.borrow().render_ref(area, buf, ctx)
    }

    fn get_device_view(&self, device: &Device, state: &State) -> Box<dyn View> {
        let device_config = if let Some(device_config) =
            state.config.device_configs.get(&device.ip.to_string())
        {
            device_config.clone()
        } else if let Some(device_config) =
            state.config.device_configs.get(&device.mac.to_string())
        {
            device_config.clone()
        } else {
            DeviceConfig {
                id: device.mac.to_string(),
                ssh_identity_file: state.config.default_ssh_identity.clone(),
                ssh_port: state.config.default_ssh_port,
                ssh_user: state.config.default_ssh_user.clone(),
            }
        };

        let device_view = DeviceView::new(
            Arc::clone(&self.dispatcher),
            device.clone(),
            device_config,
        );

        Box::new(device_view)
    }
}

impl View for DevicesView {
    fn legend(&self, state: &State) -> String {
        if let Some(view) = self.device_view.borrow().as_ref() {
            view.legend(state)
        } else {
            "(enter) manage device".into()
        }
    }

    fn override_main_legend(&self, state: &State) -> bool {
        if let Some(view) = self.device_view.borrow().as_ref() {
            view.override_main_legend(state)
        } else {
            false
        }
    }
}

impl CustomWidgetRef for DevicesView {
    fn render_ref(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) -> Result<()> {
        let view_rects =
            Layout::vertical([Constraint::Length(1), Constraint::Min(5)])
                .split(area);

        if let Some(view) = self.device_view.borrow().as_ref() {
            view.render_ref(view_rects[1], buf, ctx)
        } else {
            self.render_table(view_rects[1], buf, ctx)
        }
    }
}

impl EventHandler for DevicesView {
    fn process_event(
        &self,
        evt: &Event,
        ctx: &CustomEventContext,
    ) -> Result<bool> {
        if let Some(device_view) = self.device_view.borrow().as_ref() {
            let handled = device_view.process_event(evt, ctx)?;
            if handled {
                return Ok(handled);
            }
        }

        match evt {
            Event::FocusGained => {}
            Event::FocusLost => {}
            Event::Mouse(m) => {
                if m.kind == MouseEventKind::ScrollDown {
                    if !ctx.state.device_map.is_empty() {
                        self.next();
                    }
                    return Ok(true);
                }

                if m.kind == MouseEventKind::ScrollUp {
                    if !ctx.state.device_map.is_empty() {
                        self.previous();
                    }
                    return Ok(true);
                }
            }
            Event::Paste(_s) => {}
            Event::Resize(_x, _y) => {}
            Event::Key(key) => {
                if key.kind == KeyEventKind::Press {
                    match key.code {
                        KeyCode::Char('j') | KeyCode::Down => {
                            if !ctx.state.device_map.is_empty() {
                                self.next();
                            }
                            return Ok(true);
                        }
                        KeyCode::Char('k') | KeyCode::Up => {
                            if !ctx.state.device_map.is_empty() {
                                self.previous();
                            }
                            return Ok(true);
                        }
                        KeyCode::Enter => {
                            if let Some(selected) =
                                self.table.borrow().selected()
                            {
                                self.set_selected(selected, ctx.state);
                                return Ok(true);
                            }
                        }
                        KeyCode::Esc => {
                            let selected =
                                { self.selected_device.borrow().clone() };
                            if selected.is_some() {
                                *self.selected_device.borrow_mut() = None;
                                *self.device_view.borrow_mut() = None;
                                return Ok(true);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(false)
    }
}

#[cfg(test)]
#[path = "./devices_tests.rs"]
mod tests;
