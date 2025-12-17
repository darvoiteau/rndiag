use std::io::{self, stdout};
use std::time::Duration;
use std::cmp;

use crossterm::{
    execute,
    terminal::{EnterAlternateScreen, enable_raw_mode},
    event::{self, Event, KeyCode},
};

use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    prelude::*,
};

//Ratatui Sparklines object
pub struct Sparklines {
    pub title: &'static str,
    pub border: Borders,
    pub data: Vec<u64>,
    pub bcolor: Color,
}

//Method of Sparklines object
impl Sparklines {
    pub fn new(title: &'static str, border: Borders, data: Vec<u64>, bcolor: Color) -> Self {
        Self { title, border, data, bcolor }
    }
}


//Tuple of view => To see different view in graph, max values latencies, moy values latencies, min values latencies
#[allow(non_camel_case_types)]
enum View {
    Trend_min,
    Trend_moy,
    Trend_max
}

//Raw terminal object
struct TerminalCleanup;
impl Drop for TerminalCleanup{
    fn drop(&mut self){
        let _ = crossterm::terminal::disable_raw_mode();
        let _ = crossterm::execute!(std::io::stdout(), crossterm::terminal::LeaveAlternateScreen);
    }
}





//Main function for graph building layouting and displaying
pub fn graph_display(latency_min_sampled: &Vec<u64>, latency_moy_sampled: &Vec<u64>, latency_max_sampled: &Vec<u64>) -> Result <(), io::Error>{
    
    // Enables terminal raw mode to capture keyboard input
    enable_raw_mode()?;
    let mut stdout = stdout();
     // Switches to an alternative screen (full screen, without affecting the main terminal)
    execute!(stdout, EnterAlternateScreen)?;
    let _cleanup = TerminalCleanup;
    // Initializes the terminal with the Crossterm backend
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Sets default view as moy graph
    let mut current_view = View::Trend_moy;

        
    #[allow(deprecated)]
    loop {
    // Build interface content according to the active view
    terminal.draw(|f| {
    let size = f.size();

    // Main Layout
    let layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(90), Constraint::Percentage(10)])
        .split(size);

    //Define each view
    let (title, color, data_ref) = match current_view {
        View::Trend_min => ("Trend graph: Min latency (ms)", Color::Cyan, latency_min_sampled),
        View::Trend_moy => ("Trend graph: Avg latency (ms)", Color::Green, latency_moy_sampled),
        View::Trend_max => ("Trend graph: Max latency (ms)", Color::Red, latency_max_sampled),
    };

    //Display no data on the graph if we have no data
    if data_ref.is_empty() {
        let no_data = Paragraph::new("No data to display")
            .block(Block::default().borders(Borders::ALL).title(title))
            .style(Style::default().fg(Color::Gray));
        f.render_widget(no_data, layout[0]);
    } else {
        //Available height for the bar in the chart
        let graph_height = layout[0].height as u64;

        //Min is 0ms for the min scale in the graph. The value is used to calculate dynamically the lowest scale in the graph depending of pings values latencies
        let min_val: u64 = 0;
        let raw_max = *data_ref.iter().max().unwrap_or(&1);
        let range = cmp::max(1, raw_max - min_val);

        // Graph Lines preparation
        let mut lines = vec![String::new(); graph_height as usize];

        for &val in data_ref {
            // Calculating the proportional height
            let mut bar_height = ((val - min_val) * graph_height / range) as usize;

            //Force the minimal height of each bar to 1 for no null value
            if bar_height == 0 {
                bar_height = 1;
            }

            // Draw the bar from bottom to top
            for row in 0..graph_height as usize {
                let char_to_push = if row >= graph_height as usize - bar_height { '█' } else { ' ' };
                lines[row].push(char_to_push);
            }
        }

        let graph_text = lines.join("\n");

        // Y Scale aligned with the graph
        let scale_labels: Vec<String> = (0..=graph_height as usize)
            .rev()
            .map(|i| {
                let val = min_val + range * i as u64 / graph_height;
                format!("{:>4}ms", val)
            })
            .collect();
        let scale_text = scale_labels.join("\n");

        //Ratatui widget creation for the scale
        let scale_widget = Paragraph::new(scale_text)
            .block(Block::default().borders(Borders::NONE))
            .style(Style::default().fg(Color::Gray));

        //Ratatui widget creation for the graph
        let graph_widget = Paragraph::new(graph_text)
            .block(Block::default().title(title).borders(Borders::ALL))
            .style(Style::default().fg(color));

        // Layout horizontal : [scale][graph]
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(6), Constraint::Min(0)])
            .split(layout[0]);

        //Ratatui widget graph + scale displaying
        f.render_widget(scale_widget, chunks[0]);
        f.render_widget(graph_widget, chunks[1]);
    }

    // User Help
    let help = Paragraph::new(Line::from(Span::raw(
        "q: quit    a: min view    m: avg view    i: max view",
    )))
    .block(Block::default().borders(Borders::ALL).title("Options"));
    f.render_widget(help, layout[1]);
})?;



        
    
    // Keyboard input management (events)
    if event::poll(Duration::from_millis(500))? {
            if let Event::Key(key) = event::read()? {
                //Change view depending of the pressed key or quit the graph
                match key.code {
                    KeyCode::Char('q') => break,
                    KeyCode::Char('a') => current_view = View::Trend_min,
                    KeyCode::Char('m') => current_view = View::Trend_moy,
                    KeyCode::Char('i') => current_view = View::Trend_max,
                        _ => {}
                }
            }
        }
    }

    Ok(())


}