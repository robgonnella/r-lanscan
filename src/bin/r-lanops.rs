use std::{error::Error, io};

use ratatui::{
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{Constraint, Layout, Margin, Rect},
    style::{self, Color, Modifier, Style, Stylize},
    text::{Line, Text},
    widgets::{
        Block, BorderType, Cell, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Table, TableState,
    },
    Frame, Terminal,
};
use style::palette::tailwind;
use unicode_width::UnicodeWidthStr;

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];

const INFO_TEXT: &str =
    "(Esc) quit | (↑) move up | (↓) move down | (→) next color | (←) previous color";

const ITEM_HEIGHT: usize = 4;

struct TableColors {
    buffer_bg: Color,
    header_bg: Color,
    header_fg: Color,
    row_fg: Color,
    selected_style_fg: Color,
    normal_row_color: Color,
    alt_row_color: Color,
    footer_border_color: Color,
}

impl TableColors {
    const fn new(color: &tailwind::Palette) -> Self {
        Self {
            buffer_bg: tailwind::SLATE.c950,
            header_bg: color.c900,
            header_fg: tailwind::SLATE.c200,
            row_fg: tailwind::SLATE.c200,
            selected_style_fg: color.c400,
            normal_row_color: tailwind::SLATE.c950,
            alt_row_color: tailwind::SLATE.c900,
            footer_border_color: color.c400,
        }
    }
}

struct Data {
    hostname: String,
    ip: String,
    mac: String,
    vendor: String,
    ports: String,
}

impl Data {
    const fn ref_array(&self) -> [&String; 5] {
        [
            &self.hostname,
            &self.ip,
            &self.mac,
            &self.vendor,
            &self.ports,
        ]
    }

    fn hostname(&self) -> &str {
        &self.hostname
    }

    fn ip(&self) -> &str {
        &self.ip
    }

    fn mac(&self) -> &str {
        &self.mac
    }

    fn vendor(&self) -> &str {
        &self.vendor
    }

    fn ports(&self) -> &str {
        &self.ports
    }
}

struct App {
    state: TableState,
    items: Vec<Data>,
    longest_item_lens: (u16, u16, u16, u16, u16), // order is (hostname, ip, mac, vendor, ports)
    scroll_state: ScrollbarState,
    colors: TableColors,
    color_index: usize,
}

impl App {
    fn new() -> Self {
        let data_vec = vec![
            Data {
            hostname: "This is a much longer hostname".to_string(),
            ip: "172.17.0.1".to_string(),
            mac: "00:00:00:00:00".to_string(),
            vendor: "Android".to_string(),
            ports: "22,2000,3000,4000,5000,6000,7000,8000,9000,10000,11000,12000,13000,14000,15000,16000,17000,18000,19000,20000".to_string(),
        },
        Data {
            hostname: "Another Machine".to_string(),
            ip: "172.17.0.2".to_string(),
            mac: "00:00:00:00:01".to_string(),
            vendor: "MacOS".to_string(),
            ports: "7000,8000,9000,10000,11000,12000,13000,14000,15000,16000,17000,18000,19000,20000".to_string(),
        }
        ];
        Self {
            state: TableState::default().with_selected(0),
            longest_item_lens: constraint_len_calculator(&data_vec),
            scroll_state: ScrollbarState::new((data_vec.len() - 1) * ITEM_HEIGHT),
            colors: TableColors::new(&PALETTES[0]),
            color_index: 0,
            items: data_vec,
        }
    }
    pub fn next(&mut self) {
        let i = match self.state.selected() {
            Some(i) => (i + 1) % self.items.len(),
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn previous(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn next_color(&mut self) {
        self.color_index = (self.color_index + 1) % PALETTES.len();
    }

    pub fn previous_color(&mut self) {
        let count = PALETTES.len();
        self.color_index = (self.color_index + count - 1) % count;
    }

    pub fn set_colors(&mut self) {
        self.colors = TableColors::new(&PALETTES[self.color_index]);
    }
}

fn monitor_network() {}

fn main() -> Result<(), Box<dyn Error>> {
    // start monitoring network
    monitor_network();

    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // create app and run it
    let app = App::new();
    let res = run_app(&mut terminal, app);

    // restore terminal
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}

fn run_app<B: Backend>(terminal: &mut Terminal<B>, mut app: App) -> io::Result<()> {
    loop {
        terminal.draw(|f| ui(f, &mut app))?;

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Press {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => return Ok(()),
                    KeyCode::Char('j') | KeyCode::Down => app.next(),
                    KeyCode::Char('k') | KeyCode::Up => app.previous(),
                    KeyCode::Char('l') | KeyCode::Right => app.next_color(),
                    KeyCode::Char('h') | KeyCode::Left => app.previous_color(),
                    _ => {}
                }
            }
        }
    }
}

fn ui(f: &mut Frame, app: &mut App) {
    let rects = Layout::vertical([Constraint::Min(5), Constraint::Length(3)]).split(f.area());

    app.set_colors();

    render_table(f, app, rects[0]);

    render_scrollbar(f, app, rects[0]);

    render_footer(f, app, rects[1]);
}

fn render_table(f: &mut Frame, app: &mut App, area: Rect) {
    let header_style = Style::default()
        .fg(app.colors.header_fg)
        .bg(app.colors.header_bg);
    let selected_style = Style::default()
        .add_modifier(Modifier::REVERSED)
        .fg(app.colors.selected_style_fg);

    let header: Row<'_> = ["Hostname", "IP", "MAC", "Vendor", "Ports"]
        .into_iter()
        .map(Cell::from)
        .collect::<Row>()
        .style(header_style)
        .height(1);
    let rows = app.items.iter().enumerate().map(|(i, data)| {
        let color = match i % 2 {
            0 => app.colors.normal_row_color,
            _ => app.colors.alt_row_color,
        };
        let item = data.ref_array();
        item.into_iter()
            .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
            .collect::<Row>()
            .style(Style::new().fg(app.colors.row_fg).bg(color))
            .height(4)
    });
    let spacer = "  ";
    let t = Table::new(
        rows,
        [
            // + 1 is for padding.
            Constraint::Length(app.longest_item_lens.0 + 1),
            Constraint::Min(app.longest_item_lens.1 + 1),
            Constraint::Min(app.longest_item_lens.2 + 1),
            Constraint::Min(app.longest_item_lens.3 + 1),
            Constraint::Min(app.longest_item_lens.4 + 1),
        ],
    )
    .header(header)
    .highlight_style(selected_style)
    .highlight_symbol(Text::from(vec![
        "".into(),
        spacer.into(),
        spacer.into(),
        "".into(),
    ]))
    .bg(app.colors.buffer_bg)
    .highlight_spacing(HighlightSpacing::Always);
    f.render_stateful_widget(t, area, &mut app.state);
}

fn constraint_len_calculator(items: &[Data]) -> (u16, u16, u16, u16, u16) {
    let hostname_len = items
        .iter()
        .map(Data::hostname)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let ip_len = items
        .iter()
        .map(Data::ip)
        .flat_map(str::lines)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let mac_len = items
        .iter()
        .map(Data::mac)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let vendor_len = items
        .iter()
        .map(Data::vendor)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);
    let ports_len = items
        .iter()
        .map(Data::ports)
        .map(UnicodeWidthStr::width)
        .max()
        .unwrap_or(0);

    #[allow(clippy::cast_possible_truncation)]
    (
        hostname_len as u16,
        ip_len as u16,
        mac_len as u16,
        vendor_len as u16,
        ports_len as u16,
    )
}

fn render_scrollbar(f: &mut Frame, app: &mut App, area: Rect) {
    f.render_stateful_widget(
        Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(None)
            .end_symbol(None),
        area.inner(Margin {
            vertical: 1,
            horizontal: 1,
        }),
        &mut app.scroll_state,
    );
}

fn render_footer(f: &mut Frame, app: &App, area: Rect) {
    let info_footer = Paragraph::new(Line::from(INFO_TEXT))
        .style(Style::new().fg(app.colors.row_fg).bg(app.colors.buffer_bg))
        .centered()
        .block(
            Block::bordered()
                .border_type(BorderType::Double)
                .border_style(Style::new().fg(app.colors.footer_border_color)),
        );
    f.render_widget(info_footer, area);
}

#[cfg(test)]
mod tests {
    use crate::Data;

    #[test]
    fn constraint_len_calculator() {
        let test_data = vec![
            Data {
                hostname: "Emirhan Tala".to_string(),
                ip: "Cambridgelaan 6XX\n3584 XX Utrecht".to_string(),
                mac: "tala.emirhan@gmail.com".to_string(),
                vendor: "some vendor from some place".to_string(),
                ports: "22,23,24,25,26,27,28,29,2000,3000,4000,5000".to_string(),
            },
            Data {
                hostname: "".to_string(),
                ip: "this line is 31 characters long\nbottom line is 33 characters long"
                    .to_string(),
                mac: "thisemailis40caharacterslong@ratatui.com".to_string(),
                vendor: "".to_string(),
                ports: "".to_string(),
            },
        ];
        let (
            longest_hostname_len,
            longest_ip_len,
            longest_mac_len,
            longest_vendor_len,
            longest_ports_len,
        ) = crate::constraint_len_calculator(&test_data);

        assert_eq!(12, longest_hostname_len);
        assert_eq!(33, longest_ip_len);
        assert_eq!(40, longest_mac_len);
        assert_eq!(27, longest_vendor_len);
        assert_eq!(43, longest_ports_len);
    }
}
