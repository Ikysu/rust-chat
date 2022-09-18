use crossterm::{
    event::{self, Event, KeyCode, poll},
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, EnterAlternateScreen,
        LeaveAlternateScreen,
    },
};
use std::{error::Error, io, env, net::{TcpStream}};
use tui::{
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Modifier, Style},
    text::{Span, Spans, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
    Frame, Terminal,
};
use std::time::Duration;
use tui_input::backend::crossterm as input_backend;
use tui_input::Input;
use dns_lookup::lookup_host;

use std::io::prelude::*;


struct App {
    input: Input,
    messages: Vec<String>
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    let ips = lookup_host("0.0.0.0").expect("Ошибка DNS (клиент)");
    if ips.len()>0 {

        //socket
        let mut stream = TcpStream::connect(format!("{}:26537", ips[0])).expect("Bad connect");
        stream.set_nonblocking(true).expect("set_nonblocking call failed");
        if args.len() == 2 {
            stream.write_all(format!("/name {}",args[1]).as_bytes()).expect("Ник не установлен :(");
        }


        // setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;
        
        // create app and run it
        let app = App {
            input: Input::default(),
            messages: Vec::new()
        };
        let res = run_app(&mut terminal, app, stream);
        
        // restore terminal
        disable_raw_mode()?;
        execute!(
            terminal.backend_mut(),
            LeaveAlternateScreen
        )?;
        terminal.show_cursor()?;

        if let Err(err) = res {
            println!("{:?}", err)
        }
    }else{
        println!("Ошибка DNS (сервер)")
    }

    Ok(())
}


fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    mut app: App,
    mut stream: TcpStream
) -> io::Result<()> {

    //let mut incoming = listener.incoming();

    let mut buf = [0;512];
    
    loop{
        //app.messages.push(format!("UPD {}", counter).to_string());

        terminal.draw(|f| ui(f, &mut app)).unwrap();
        if poll(Duration::from_millis(0))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Enter => {
                        let text = app.input.value().to_string();
                        match stream.write_all(text.as_bytes()) {
                            Ok(_)=>{
                                app.input.reset();
                            },
                            Err(_)=>{
                                app.messages.push("Сообщение не отправленно :(".to_string());
                            }
                        };
                    }
                    KeyCode::Esc => {
                        return Ok(());
                    }
                    _ => {
                        if app.input.value().len() < 100 {
                            input_backend::to_input_request(Event::Key(key))
                            .and_then(|req| app.input.handle(req));
                        }
                    }
                }
            }
        }

        match stream.read(&mut buf) {
            Ok(_bytes_read)=>{
                let msg = format!("{}", String::from_utf8_lossy(&buf).trim()).replace("\0", "");
                for rawdt in msg.split("^") {
                    if rawdt.len() > 0 {
                        app.messages.push(rawdt.to_string());
                    }
                }
                buf.iter_mut().for_each(|x| *x = 0);
            },
            Err(e)=>{
                if e.kind() != io::ErrorKind::WouldBlock {
                    app.messages.push(format!("{:?}", e));
                }
            }
        };
    }
    
}

fn ui<B: Backend>(f: &mut Frame<B>, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints(
            [
                Constraint::Length(2),
                //Constraint::Length(1),
                Constraint::Min(3),
                Constraint::Length(3),
            ]
            .as_ref(),
        )
        .split(f.size());

    let (msg, style) = (
        vec![
            Span::raw("Press "),
            Span::styled(
                "ESC",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to exit, "),
            Span::styled(
                "Enter",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" to send and "),
            Span::styled(
                "/help",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(" for help."),
        ],
        Style::default(),
    );

    let mut text = Text::from(Spans::from(msg));
    text.patch_style(style);
    let help_message = Paragraph::new(text).block(Block::default().borders(Borders::BOTTOM));
    f.render_widget(help_message, chunks[0]);

    let width = chunks[0].width.max(3) - 3; // keep 2 for borders and 1 for cursor
    let scroll = (app.input.cursor() as u16).max(width) - width;
    let input = Paragraph::new(app.input.value())
        .scroll((0, scroll))
        .block(Block::default().borders(Borders::TOP));
    f.render_widget(input, chunks[2]);
    f.set_cursor(
        chunks[2].x + (app.input.cursor() as u16).min(width),
        chunks[2].y + 1,
    );

    let messages: Vec<String>;

    if app.messages.len()>chunks[1].height.into() {
        let n_us: usize = chunks[1].height as usize;
        //let help_msg = &app.messages[(app.messages.len()-n_us)..];
        //let help_message = Paragraph::new(format!("new {:?}", help_msg));
        //f.render_widget(help_message, chunks[1]);
        messages = app.messages[(app.messages.len()-n_us)..].into();
    }else{
        messages = app.messages[..].into()
    }

    let messages: Vec<ListItem> = messages
        .iter()
        .enumerate()
        .map(|(_, m)| {
            let content = vec![Spans::from(Span::raw(format!("{}", m)))];
            ListItem::new(content)
        })
        .collect();

    
    
    let messages = List::new(messages)
        .block(Block::default());
    f.render_widget(messages, chunks[1]);

    
}
