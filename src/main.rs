use std::{
    cmp,
    process,
    io::{self, Write, Stdout},
};
use termion::{
    terminal_size,
    raw::IntoRawMode, 
    clear, 
    cursor,
    event::Key,
    input::TermRead,
};

const BOTTOM_PADDING: u16 = 1;
const LEFT_PADDING: u16 = 7;

#[derive(Clone, Copy, Debug)]
enum Mode {
    Insert,
    Normal,
}

#[derive(Debug)]
enum Command {
    MoveUp,
    MoveDown,
    MoveLeft,
    MoveRight,
    InsertChar(char),
    NewLine,
    NewLineO,
    BackSpace,
    Delete,
    EnterInsertMode,
    EnterNormalMode,
    Quit,
    NoOp,
}

struct Line {
    chars: Vec<char>,
}
impl Line {
    fn len(&self) -> usize {
        self.chars.len()
    }
}

struct Cursor {
    row: usize,
    col: usize,
}

struct Global {
    lines: Vec<Line>,
    cursor: Cursor,
    mode: Mode,
}
impl Global {
    fn current_line(&mut self) -> &mut Line {
        &mut self.lines[self.cursor.row]
    }
    fn amount_of_lines(&self) -> usize {
        self.lines.len()
    }
    fn move_up(&mut self) {
        if self.cursor.row > 0 {
            self.cursor.row -= 1;
            self.reset_cursor();
        }    
    }
    fn move_down(&mut self) {
        if self.cursor.row + 1 < self.lines.len(){
            self.cursor.row += 1;
            self.reset_cursor();
        }
    }
    fn move_left(&mut self) {
        if self.cursor.col > 0 as usize {
            self.cursor.col -= 1;
        }
    }
    fn move_right(&mut self) {
        if self.cursor.col < self.current_line().len() {
            self.cursor.col += 1;
        }
    }
    fn reset_cursor(&mut self) {
        if self.current_line().len() < self.cursor.col {
            self.cursor.col = self.current_line().len();
        }
    }
}

struct View {
    width: u16,
    height: u16,
    //holds the start line in view currently
    start: usize,
}
impl View {
    fn update_scroll(&mut self, global: &mut Global) {
        let cur_row = global.cursor.row + 1;
        if cur_row >= self.height as usize - BOTTOM_PADDING as usize + self.start {
            self.start += 1;
        }
        else if cur_row < self.start && self.start > 1{
            self.start -= 1;
        }
    }
}

fn main() {
    let (width, height) = terminal_size().unwrap_or((20, 20));
    let mut stdout = io::stdout().into_raw_mode().unwrap_or_else(|err| {
        eprintln!("Failed to enter raw mode: {err}");
        process::exit(1);
    });

    let mut view = View {
        width,
        height,
        start: 1,
    };
    let mut global = Global { 
        lines: Vec::new(), 
        cursor: Cursor{ col: 0, row: 0 }, 
        mode: Mode::Normal 
    };
    
    //push the first line manually for now
    global.lines.push(Line{ chars: Vec::new() });

    //main loop
    loop {
        let _ = print_tui(&mut global, &mut view, &mut stdout).unwrap();

        let c = io::stdin().keys().next();
        let key = match c {
            Some(c) => match c {
                Ok(c) => c,
                Err(_) => continue,
            },
            None => continue,
        };

        let cmd: Command = map_key(key, global.mode);

        match cmd {
            Command::Quit => break,
            Command::MoveUp => {
                global.move_up();
            },
            Command::MoveDown => {
                global.move_down();
            },
            Command::MoveLeft => global.move_left(),
            Command::MoveRight => global.move_right(),
            Command::InsertChar(c) => {
                let cur_col = global.cursor.col;
                global.current_line().chars.insert(cur_col, c);
                global.move_right();
            },
            Command::EnterInsertMode => {
                global.mode = Mode::Insert;
            },
            Command::EnterNormalMode => {
                global.mode = Mode::Normal;
            },
            Command::NewLineO => {
                let cur_row = global.cursor.row;
                global.lines.insert(cur_row + 1, Line {chars: Vec::new() });
                global.move_down();
                global.mode = Mode::Insert;
            },
            Command::NewLine => {
                let cur_row = global.cursor.row;
                let cur_col = global.cursor.col;
                //fresh line
                if cur_col >= global.current_line().len() {
                    global.lines.insert(cur_row + 1, Line{ chars: Vec::new() });
                    global.move_down();
                }
                //spliced line
                else {
                    let new_line_vec = &mut global.current_line().chars.split_off(cur_col);
                    global.lines.insert(cur_row + 1, Line{ chars: Vec::new() });
                    global.move_down();
                    global.current_line().chars.append(new_line_vec); 
                }
            },
            Command::BackSpace => {
                if global.cursor.col > 0 {
                    let cur_col = global.cursor.col - 1;
                    global.current_line().chars.remove(cur_col);
                    global.move_left();
                }else {
                    let cur_row = global.cursor.row;
                    if cur_row > 0 {
                        let mut temp: Vec<char> = global.lines[cur_row].chars.clone();
                        global.lines.remove(cur_row);
                        global.move_up();
                        global.current_line().chars.append(&mut temp);
                        //TODO: Make it so we end up on the end of the line above not the
                        //start(problem in move_up fn)
                    }
                }
            },
            Command::Delete => {
                let cur_col = global.cursor.col;
                if cur_col < global.current_line().len() {
                    global.current_line().chars.remove(cur_col);
                }
            },
            Command::NoOp => continue,
        }
    }

}

fn print_tui(global: &mut Global, view: &mut View, stdout: &mut Stdout) -> io::Result<()>{
    view.update_scroll(global);
    write!(stdout, "{}", clear::All)?;
    write!(stdout, "{}", cursor::Goto(1, view.height))?;
    write!(stdout, "{:?}", global.mode)?;
    //print the view.start for debugging
    write!(stdout, "{}", cursor::Goto(10, view.height))?;
    write!(stdout, "start: {}", view.start)?;

    write!(stdout, "{}{}|{}", cursor::Goto(view.width - 5, view.height), 
        global.cursor.col as usize + 1, global.cursor.row + 1)?;
    print_lines(view, global, stdout);
    print_content(view, global, stdout);
    
    let cur_row = global.cursor.row as u16 + 1 - view.start as u16 + 1;
    write!(stdout, "{}", cursor::Goto(global.cursor.col as u16 + LEFT_PADDING + 1, 
    cur_row))?;

    stdout.flush()?;

    Ok(())
}

fn print_lines(view: &mut View, global: &mut Global, stdout: &mut Stdout) {
    let max = cmp::min(view.height - BOTTOM_PADDING, global.amount_of_lines() as u16);
    for i in 0..max as usize {
        write!(stdout, "{}", cursor::Goto(1, i as u16 + 1)).unwrap();
        write!(stdout, "{}", i + view.start).unwrap();
        write!(stdout, "{}", cursor::Goto(LEFT_PADDING as u16, i as u16 + 1)).unwrap();
        write!(stdout, "{}", '|').unwrap();
    }
}

fn print_content(view: &mut View, global: &mut Global, stdout: &mut Stdout) {
    let max = view.height as usize - BOTTOM_PADDING as usize + view.start as usize;
    for i in view.start - 1..max - 1{
        let line = match global.lines.get(i){
            Some(l) => l,
            None => break,
        };
        for (j, char) in line.chars.iter().enumerate() {
            write!(stdout, "{}", cursor::Goto(j as u16 + LEFT_PADDING + 1, i as u16 + 1 - view.start as u16 + 1)).unwrap();
            write!(stdout, "{}", char).unwrap();
        }
    }
}

fn map_key(key: Key, mode: Mode) -> Command {
    match mode {
        Mode::Normal => {
            match key {
                Key::Char('q') => Command::Quit,
                Key::Char('k') => Command::MoveUp,
                Key::Char('j') => Command::MoveDown,
                Key::Char('h') => Command::MoveLeft,
                Key::Char('l') => Command::MoveRight,
                Key::Char('a') => Command::EnterInsertMode,
                Key::Char('x') => Command::Delete,
                Key::Char('o') => Command::NewLineO,
                _ => Command::NoOp,
            }
        },
        Mode::Insert => {
            match key {
                Key::Char('\n') => Command::NewLine,
                Key::Backspace => Command::BackSpace,
                Key::Esc => Command::EnterNormalMode,
                Key::Char(char) => Command::InsertChar(char),
                _ => Command::NoOp,
            }
        },
    }
} 
