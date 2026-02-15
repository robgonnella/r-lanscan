//! Devices list view showing all discovered network devices in a table.

use color_eyre::eyre::Result;
use itertools::Itertools;
use r_lanlib::scanners::Device;
use ratatui::{
    crossterm::event::{
        Event, KeyCode, KeyEventKind, MouseButton, MouseEventKind,
    },
    layout::{Constraint, Layout, Rect},
};
use std::cell::RefCell;

use crate::{
    config::DeviceConfig,
    store::state::State,
    ui::{
        components::table::{DEFAULT_ITEM_HEIGHT, Table},
        views::{device::DeviceView, traits::CustomEventContext},
    },
};

use super::traits::{CustomWidgetContext, CustomWidgetRef, EventHandler, View};

/// Main view showing all discovered devices in a selectable table.
pub struct DevicesView {
    selected_device: RefCell<Option<(Device, DeviceConfig)>>,
    device_view: RefCell<Option<RefCell<DeviceView>>>,
    table: RefCell<Table>,
    table_area: RefCell<Option<Rect>>,
}

impl DevicesView {
    /// Creates a new devices view with the given dispatcher.
    pub fn new() -> Self {
        Self {
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
            table_area: RefCell::new(None),
        }
    }

    fn next(&self) {
        self.table.borrow_mut().next();
    }

    fn previous(&self) {
        self.table.borrow_mut().previous();
    }

    fn get_device_config(
        &self,
        device: &Device,
        state: &State,
    ) -> DeviceConfig {
        if let Some(dev_conf) =
            state.config.device_configs.get(&device.mac.to_string())
        {
            dev_conf.clone()
        } else {
            DeviceConfig {
                id: device.mac.to_string(),
                ssh_identity_file: state.config.default_ssh_identity.clone(),
                ssh_port: state.config.default_ssh_port,
                ssh_user: state.config.default_ssh_user.clone(),
            }
        }
    }

    fn set_selected(&self, i: usize, state: &State) -> Result<()> {
        let list = state.device_list();

        if !list.is_empty() && i < list.len() {
            let device = list[i].clone();
            let device_config = self.get_device_config(&device, state);
            self.selected_device
                .replace(Some((device.clone(), device_config.clone())));
            let device_view = DeviceView::new(device, device_config);
            self.device_view.replace(Some(RefCell::new(device_view)));
        }

        Ok(())
    }

    fn render_table(
        &self,
        area: Rect,
        buf: &mut ratatui::prelude::Buffer,
        ctx: &CustomWidgetContext,
    ) -> Result<()> {
        // Store the table area for mouse click handling
        self.table_area.replace(Some(area));

        let devices = ctx.state.device_list();

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
}

impl View for DevicesView {
    fn legend(&self, state: &State) -> String {
        if let Some(view) = self.device_view.borrow().as_ref() {
            view.borrow().legend(state)
        } else {
            "(enter) manage device".into()
        }
    }

    fn override_main_legend(&self, state: &State) -> bool {
        if let Some(view) = self.device_view.borrow().as_ref() {
            view.borrow().override_main_legend(state)
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
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)])
                .split(area);

        if let Some(view) = self.device_view.borrow().as_ref()
            && let Some(selected) = self.selected_device.borrow().as_ref()
            && let Some(device) = ctx.state.device_map.get(&selected.0.ip)
        {
            // scoped to drop mutable borrow before borrowing again
            {
                let device_config = self.get_device_config(device, ctx.state);
                view.borrow_mut()
                    .update_device(device.to_owned(), device_config);
            }
            view.borrow().render_ref(view_rects[1], buf, ctx)
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
            let handled = device_view.borrow().process_event(evt, ctx)?;
            if handled {
                return Ok(handled);
            }
        }

        match evt {
            Event::FocusGained => {}
            Event::FocusLost => {}
            Event::Mouse(mouse) => {
                // Handle scroll events
                match mouse.kind {
                    MouseEventKind::ScrollDown => {
                        if !ctx.state.device_map.is_empty() {
                            self.next();
                            return Ok(true);
                        }
                    }
                    MouseEventKind::ScrollUp => {
                        if !ctx.state.device_map.is_empty() {
                            self.previous();
                            return Ok(true);
                        }
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        // Handle click to select row
                        if let Some(area) = *self.table_area.borrow() {
                            // Calculate clicked row, dropping borrow before select
                            let row_idx_opt = {
                                self.table
                                    .borrow()
                                    .calculate_clicked_row(mouse.row, area)
                            };

                            if let Some(row_idx) = row_idx_opt {
                                self.table.borrow_mut().select(row_idx);
                                return Ok(true);
                            }
                        }
                    }
                    _ => {}
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
                                self.set_selected(selected, ctx.state)?;
                                return Ok(true);
                            }
                        }
                        KeyCode::Esc => {
                            if self.selected_device.take().is_some() {
                                self.device_view.replace(None);
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
