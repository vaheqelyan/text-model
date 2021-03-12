extern crate freetype as ft;
extern crate glfw;

use glfw::{Action, Context, Key};
use num_traits::{clamp, clamp_max, clamp_min, sign};
use std::collections::HashMap;
use std::sync::mpsc::*;
use std::thread::Builder;

#[derive(Debug)]
struct BorderSize {
    width: i32,
    height: i32,
}

#[derive(Debug)]
struct FontSize {
    width: i64,
    height: i64,
    advance: i64,
}

#[derive(Debug)]
enum KeyCommand {
    Left,
    Right,
    Back,
    Value(String),
    NewLine,
    Size(i32, i32),
}

fn create_font_map(font: &str) -> HashMap<char, FontSize> {
    // Freetype get measurements
    let library = ft::Library::init().unwrap();
    let face = library.new_face(font, 0).unwrap();
    face.set_char_size(40 * 64, 0, 50, 0).unwrap();

    let string = String::from("!\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ[\\]^_`abcdefghijklmnopqrstuvwxyz{|}~");

    let mut font_measure: HashMap<char, FontSize> = HashMap::new();

    for c in string.chars() {
        face.load_char(c as usize, ft::face::LoadFlag::DEFAULT)
            .unwrap();
        let get_metrics = face.glyph().metrics();
        let get_advance = face.glyph().advance();
        font_measure.insert(
            c,
            FontSize {
                width: get_metrics.width >> 6,
                height: get_metrics.height >> 6,
                advance: get_advance.x >> 6,
            },
        );
    }
    font_measure
}

fn get_split(
    value: &String,
    get_sizes: &HashMap<char, FontSize>,
    border_size: &BorderSize,
) -> Vec<String> {
    let mut total = 0;
    let mut result: Vec<String> = vec![String::with_capacity(10)];
    let mut cursor = 0;
    let mut start = 0;

    value.chars().fold(&mut result, |acc, value| {
        let measure = get_sizes.get(&value).unwrap();

        if total + measure.advance > border_size.width as i64 {
            //acc.push(String::from(value));
            acc.push(String::with_capacity(10));

            cursor += 1;
            total = 0;
        } else {
            acc[cursor].push(value);
            total += measure.advance;
        }

        acc
    });

    result
}

fn create_wrapped_buffer(
    rows: &Vec<String>,
    get_font_size: &HashMap<char, FontSize>,
    get_border_size: &BorderSize,
) -> Vec<String> {
    let mut buffer: Vec<String> = vec![];
    //println!("ROWS {:?}", rows);

    rows.iter()
        .enumerate()
        .fold(&mut buffer, |mut acc, (index, value)| {
            let mut split = get_split(&value, get_font_size, get_border_size);
            println!("S {:?}",split);

            acc.append(&mut split);
            acc
        });

    buffer
}

#[derive(Debug)]
struct Cursor {
    x: usize,
    y: usize,
}

fn do_validate_cursor(rows: &Vec<String>, cursor: &Cursor) -> Cursor {
    if rows[cursor.y].len() < cursor.x {
        Cursor {
            x: 0,
            y: cursor.y + 1,
        }
    } else {
        Cursor {
            x: cursor.x,
            y: cursor.y,
        }
    }
}

// Operations
fn mut_type(cursor: &Cursor, buffer: &mut Vec<String>, value: &String) -> Cursor {
    let mut get_line = &mut buffer[cursor.y];
    get_line.insert_str(cursor.x, value);
    Cursor {
        x: cursor.x + 1,
        y: cursor.y,
    }
}

fn mut_new_line(cursor: &Cursor, buffer: &mut Vec<String>) -> Cursor {
    buffer.push(String::with_capacity(3));
    Cursor {
        y: cursor.y + 1,
        x: 0,
    }
}

fn mut_backspace(cursor: &Cursor, buffer: &mut Vec<String>) -> Cursor {
    let mut get_line = &mut buffer[cursor.y];
    get_line.remove(cursor.x - 1);
    Cursor {
        x: cursor.x - 1,
        y: cursor.y,
    }
}

fn mut_delete_line(cursor: &Cursor, buffer: &mut Vec<String>) -> Cursor {
    buffer.remove(cursor.y);
    let get_back_line = cursor.y - 1;
    let get_line_length = &buffer[get_back_line].len();
    Cursor {
        x: *get_line_length,
        y: get_back_line,
    }
    /*cursor.y -= 1;

    let mut get_line = &mut rows[cursor.y];
    cursor.x = get_line.len();*/
}

fn text_model(recv: Receiver<KeyCommand>) {
    let mut rows: Vec<String> = vec![String::with_capacity(3)];
    //let mut cursor_x = 0;
    //let mut cursor_y = 0;

    let mut cursor = Cursor { x: 0, y: 0 };

    let mut border_size = BorderSize {
        width: 0,
        height: 0,
    };

    let font = "./Lato-Regular.ttf";
    let mut font_measure: HashMap<char, FontSize> = create_font_map(&font);

    loop {
        let value = recv.recv();

        match value.unwrap() {
            KeyCommand::Value(string) => {
                let new_cursor = mut_type(&cursor, &mut rows, &string);
                cursor = new_cursor;
            }
            KeyCommand::NewLine => {
                //let new_cursor = mut_new_line(&cursor, &mut rows);
                //cursor = new_cursor;
            }
            KeyCommand::Left => {
                if cursor.x > 0 {
                    cursor.x -= 1;
                }
            }
            KeyCommand::Right => {
                if cursor.x < 10 {
                    cursor.x += 1;
                }
            }
            KeyCommand::Back => {
                if cursor.x > 0 {
                    let new_cursor = mut_backspace(&cursor, &mut rows);
                    cursor = new_cursor;
                } else {
                    let get_cursor = mut_delete_line(&cursor, &mut rows);
                    let new_cursor = mut_backspace(&get_cursor, &mut rows);
                    cursor = new_cursor;
                }
            }
            KeyCommand::Size(width, height) => {
                border_size = BorderSize { width, height };
            }
            _ => (),
        }

        let mut get_new_buffer = create_wrapped_buffer(&rows, &font_measure, &border_size);
        rows.clear();
        rows.append(&mut get_new_buffer);

        //println!("{:?}",get_split(&String::from("HELLOWORLDCANIHEL"), &font_measure, &border_size));
        //HELLOWORLDCANIHEL


        let get_new_cursor = do_validate_cursor(&rows, &cursor); // NOTE: UNSAFE OPERATION
        //println!("{:?}", rows);

        cursor = get_new_cursor; // NOTE: SIDE EFFECT
    }
}

fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();

    let (mut window, events) = glfw
        .create_window(300, 300, "Hello this is window", glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window.");

    window.set_all_polling(true);
    window.make_current();

    let (send, recv): (Sender<KeyCommand>, Receiver<KeyCommand>) = channel();

    let layout_task = Builder::new().name("layout task".to_string());

    let layout_task_done = layout_task.spawn(move || {
        text_model(recv);
    });

    while !window.should_close() {
        glfw.poll_events();
        for (_, event) in glfw::flush_messages(&events) {
            handle_window_event(&mut window, event, &send);
        }
    }
}

fn handle_window_event(
    window: &mut glfw::Window,
    event: glfw::WindowEvent,
    sender: &Sender<KeyCommand>,
) {
    match event {
        glfw::WindowEvent::Key(Key::Escape, _, Action::Press, _) => window.set_should_close(true),
        glfw::WindowEvent::Char(character) => {
            sender.send(KeyCommand::Value(character.to_string()));
        }
        glfw::WindowEvent::FramebufferSize(w, h) => {
            sender.send(KeyCommand::Size(w, h));
        }
        glfw::WindowEvent::Key(key, _, keymod, _) => match key {
            glfw::Key::Enter => match keymod {
                Action::Repeat | Action::Press => {
                    sender.send(KeyCommand::NewLine);
                }
                _ => (),
            },
            glfw::Key::Backspace => match keymod {
                Action::Repeat | Action::Press => {
                    sender.send(KeyCommand::Back);
                }
                _ => (),
            },
            glfw::Key::Left => match keymod {
                Action::Repeat | Action::Press => {
                    sender.send(KeyCommand::Left);
                }
                _ => (),
            },
            glfw::Key::Right => match keymod {
                Action::Repeat | Action::Press => {
                    sender.send(KeyCommand::Right);
                }
                _ => (),
            },
            _ => (),
        },
        _ => {}
    }
}
