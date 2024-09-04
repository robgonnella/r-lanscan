use core::time;
use std::{
    error::Error,
    io,
    sync::{Arc, RwLock},
};

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

const ELLIPSIS: &str = "…";

const COLUMN_WIDTH: u16 = 50;

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

pub struct Data {
    pub hostname: String,
    pub ip: String,
    pub mac: String,
    pub vendor: String,
    pub ports: String,
}

impl Data {
    fn ref_array(&self, col_width: usize) -> [String; 5] {
        let hostname_len = self.hostname.width();
        let ip_len = self.ip.width();
        let mac_len = self.mac.width();
        let vendor_len = self.mac.width();
        let ports_len = self.ports.width();

        let mut hostname = self.hostname.to_owned();
        if hostname_len >= col_width {
            hostname.truncate(col_width - 10);
            hostname.push_str(ELLIPSIS);
        }
        let mut ip = self.ip.to_owned();
        if ip_len >= col_width {
            ip.truncate(col_width - 10);
            ip.push_str(ELLIPSIS);
        }
        let mut mac = self.mac.to_owned();
        if mac_len >= col_width {
            mac.truncate(col_width - 10);
            mac.push_str(ELLIPSIS);
        }
        let mut vendor = self.vendor.to_owned();
        if vendor_len >= col_width {
            vendor.truncate(col_width - 10);
            vendor.push_str(ELLIPSIS);
        }
        let mut ports = self.ports.to_owned();
        if ports_len >= col_width {
            ports.truncate(col_width - 10);
            ports.push_str(ELLIPSIS);
        }
        [hostname, ip, mac, vendor, ports]
    }
}

struct App {
    state: TableState,
    items: Arc<RwLock<Vec<Data>>>,
    scroll_state: ScrollbarState,
    colors: TableColors,
    color_index: usize,
}

impl App {
    fn new(data_set: Arc<RwLock<Vec<Data>>>) -> Self {
        let set = data_set.read().unwrap();
        let mut height = ITEM_HEIGHT;
        if set.len() > 0 {
            height = (set.len() - 1) * ITEM_HEIGHT;
        }
        Self {
            state: TableState::default().with_selected(0),
            scroll_state: ScrollbarState::new(height),
            colors: TableColors::new(&PALETTES[0]),
            color_index: 0,
            items: Arc::clone(&data_set),
        }
    }

    pub fn next(&mut self) {
        let data = self.items.read().unwrap();

        let i = match self.state.selected() {
            Some(i) => (i + 1) % data.len(),
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn previous(&mut self) {
        let data = self.items.read().unwrap();

        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    data.len() - 1
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

pub fn launch(data_set: Arc<RwLock<Vec<Data>>>) -> Result<(), Box<dyn Error>> {
    // setup terminal
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let app = App::new(Arc::clone(&data_set));
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
        {
            let set = app.items.read().unwrap();
            let mut height = ITEM_HEIGHT;
            if set.len() > 0 {
                height = (set.len() - 1) * ITEM_HEIGHT;
            }
            app.scroll_state = ScrollbarState::new(height);
        }

        terminal.draw(|f| ui(f, &mut app))?;

        // Use poll here so we don't block the thread, this will allow
        // rendering of incoming device data from network as it's received
        if let Ok(has_event) = event::poll(time::Duration::from_secs(1)) {
            if has_event {
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

    let items = app.items.read().unwrap();

    let rows = items.iter().enumerate().map(|(i, data)| {
        let color = match i % 2 {
            0 => app.colors.normal_row_color,
            _ => app.colors.alt_row_color,
        };
        let col_width = area.width / 5;
        let item = data.ref_array(col_width as usize);
        item.into_iter()
            .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
            .collect::<Row>()
            .style(Style::new().fg(app.colors.row_fg).bg(color))
            .height(ITEM_HEIGHT as u16)
    });
    let spacer = "  ";
    let t = Table::new(
        rows,
        [
            Constraint::Max(COLUMN_WIDTH),
            Constraint::Max(COLUMN_WIDTH),
            Constraint::Max(COLUMN_WIDTH),
            Constraint::Max(COLUMN_WIDTH),
            Constraint::Max(COLUMN_WIDTH),
        ],
    )
    .header(header)
    .highlight_style(selected_style)
    .highlight_symbol(Text::from(vec![
        spacer.into(),
        spacer.into(),
        spacer.into(),
        spacer.into(),
    ]))
    .bg(app.colors.buffer_bg)
    .highlight_spacing(HighlightSpacing::Always);
    f.render_stateful_widget(t, area, &mut app.state);
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
