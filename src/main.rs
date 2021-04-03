mod font_loader;
mod opengl;

use font_loader::{create_font_map, FontSize};

use dela::{delaunay, segment};
extern crate freetype as ft;
extern crate glfw;

use std::sync::{Arc, Mutex};

use glfw::{Action, Context, Key};
use std::collections::HashMap;
use std::sync::mpsc::*;
use std::thread::Builder;

#[derive(Debug)]
struct BorderSize {
    width: i32,
    height: i32,
}

#[derive(Debug)]
enum KeyCommand {
    Left,
    Right,
    Back,
    Value(String),
    NewLine,
    Size(i32, i32),
    None,
}

fn get_split(
    value: &String,
    get_sizes: &HashMap<char, FontSize>,
    border_size: &BorderSize,
    cursor_distance: usize,
    current_cursor: &Cursor,
) -> (Vec<String>, Cursor) {
    let mut total = 0;
    let mut cursor = 0;
    let mut start = 0;

    let fold_res = value.chars().enumerate().fold(
        (vec![String::with_capacity(10)], Cursor { x: 0, y: 0 }),
        |mut acc, (index, value)| {
            let measure = get_sizes.get(&value).unwrap();

            if total + measure.advance > border_size.width as i64 {
                acc.0.push(String::from(value));
                cursor += 1;
                total = 0;
            } else {
                acc.0[cursor].push(value);
                total += measure.advance;
            }

            if cursor_distance > index {
                acc.1 = Cursor {
                    x: acc.0[cursor].chars().count(),
                    y: cursor,
                };
            }

            acc
        },
    );

    fold_res
}

fn normalize(rows: &Vec<Buf>) -> Vec<Buf> {
    let mut buffer: Vec<Buf> = vec![];

    rows.iter().fold(&mut buffer, |mut acc, value| {
        match value.link {
            Some(index) => {
                let mut get_last = &mut acc.last_mut().unwrap();
                //println!("{:?}", get_last.text);
                get_last.text.push_str(&value.text);
            }
            None => {
                acc.push(Buf {
                    text: value.text.clone(),
                    link: None,
                    line: None,
                });
            }
        }
        acc
    });
    buffer
}

fn get_cursor_distance(rows: &Vec<Buf>, cursor: &Cursor) -> (usize, usize, usize) {
    let get_current_line = rows[cursor.y].line;

    let start_index = rows
        .iter()
        .position(|x| x.line == get_current_line)
        .unwrap();

    let left_size = rows
        .into_iter()
        .enumerate()
        .filter(|(pos, value)| {
            if pos >= &start_index && pos < &cursor.y {
                true
            } else {
                false
            }
        })
        .map(|x| x.1)
        .fold(0, |acc, value| acc + value.text.chars().count());

    (left_size + cursor.x, get_current_line.unwrap(), start_index)
}

fn create_wrapped_buffer(
    rows: &Vec<Buf>,
    get_font_size: &HashMap<char, FontSize>,
    get_border_size: &BorderSize,
    cursor: &Cursor,
    temp_cursor: &TempCursor,
) -> (Vec<Buf>, Cursor) {
    let mut buffer: Vec<Buf> = vec![];

    let normalize_buffer = normalize(&rows);

    let nc = Cursor {
        x: temp_cursor.x + cursor.x,
        y: temp_cursor.y + cursor.y,
    };

    let (get_distance, active_line, start_index) = get_cursor_distance(&rows, &nc);

    let result = normalize_buffer.iter().enumerate().fold(
        (vec![], Cursor { x: 0, y: cursor.y }),
        |mut acc, (index, value)| {
            let (mut split, new_cursor) = get_split(
                &value.text,
                get_font_size,
                get_border_size,
                get_distance,
                cursor,
            );

            if index == active_line {
                acc.1.x = new_cursor.x;
                acc.1.y = acc.0.len() + new_cursor.y; //+ new_cursor.y;
            }

            let mut foo: Vec<Buf> = split
                .iter()
                .enumerate()
                .map(|(pos, x)| Buf {
                    text: x.to_string(),
                    link: if pos == 0 { None } else { Some(index) },
                    line: Some(index),
                })
                .collect();

            acc.0.append(&mut foo);
            acc
        },
    );

    result
}

#[derive(Debug)]
struct Cursor {
    x: usize,
    y: usize,
}

#[derive(Debug)]
struct TempCursor {
    x: usize,
    y: usize,
}

// Operations
fn mut_type(cursor: &Cursor, buffer: &mut Vec<Buf>, value: &String) -> Cursor {
    let mut get_line = &mut buffer[cursor.y];
    get_line.text.insert_str(cursor.x, value);
    Cursor {
        x: cursor.x + 1,
        y: cursor.y,
    }
}

fn mut_new_line(cursor: &Cursor, buffer: &mut Vec<Buf>) -> (TempCursor, Cursor) {
    let (_, active_line, _) = get_cursor_distance(&buffer, &cursor);
    buffer.push(Buf {
        text: String::with_capacity(3),
        link: None,
        line: Some(active_line + 1),
    });

    (TempCursor { y: 1, x: 0 }, Cursor { y: cursor.y, x: 0 })
}

fn mut_backspace(cursor: &Cursor, buffer: &mut Vec<Buf>) -> Cursor {
    let mut get_line = &mut buffer[cursor.y];
    get_line.text.remove(cursor.x - 1);
    Cursor {
        x: cursor.x - 1,
        y: cursor.y,
    }
}

fn mut_delete_line(cursor: &Cursor, buffer: &mut Vec<Buf>) -> Cursor {
    buffer.remove(cursor.y);
    let get_back_line = cursor.y - 1;
    let get_line_length = &buffer[get_back_line].text.len();
    Cursor {
        x: *get_line_length,
        y: get_back_line,
    }
}

fn get_row_len(buffer: &Vec<Buf>, cursor: &Cursor) {}

#[derive(Debug, Clone)]
struct Buf {
    text: String,
    link: Option<usize>,
    line: Option<usize>,
}

fn text_model(
    recv: Receiver<KeyCommand>,
    shared_buffer: Arc<Mutex<Vec<Buf>>>,
    send_back: Sender<bool>,
    font_measure: HashMap<char, FontSize>,
) {
    let mut rows: Vec<Buf> = vec![Buf {
        text: String::from(""),
        link: None,
        line: Some(1),
    }];

    let mut cursor = Cursor { x: 0, y: 0 };
    let mut temp_cursor = TempCursor { x: 0, y: 0 };

    let mut border_size = BorderSize {
        width: 0,
        height: 0,
    };

    let mut cmd: KeyCommand = KeyCommand::None;

    loop {
        let value = recv.recv();

        match value.unwrap() {
            KeyCommand::Value(string) => {
                //shared_buffer.lock().unwrap().push(Buf {line:None,link:None,text: "".to_string()});

                let new_cursor = mut_type(&cursor, &mut rows, &string);
                cursor = new_cursor;
                cmd = KeyCommand::Value("z".to_string());
                temp_cursor = TempCursor { x: 0, y: 0 };
            }
            KeyCommand::NewLine => {
                let (new_temp_cursor, new_cursor) = mut_new_line(&cursor, &mut rows);
                cursor = new_cursor;
                temp_cursor = new_temp_cursor;
            }
            KeyCommand::Left => {
                //if cursor.x > 0 {
                cursor.x -= 1;
                //}
            }
            KeyCommand::Right => {
                //let get_current_row_len = get_row_len(&rows, &cursor);
                //if cursor.x < 10 {
                //cursor.x += 1;
                //}
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
                cmd = KeyCommand::Back;
            }
            KeyCommand::Size(width, height) => {
                border_size = BorderSize { width, height };
            }
            _ => (),
        }

        let (mut get_new_buffer, new_cursor) =
            create_wrapped_buffer(&rows, &font_measure, &border_size, &cursor, &temp_cursor);
        rows.clear();
        rows.append(&mut get_new_buffer);

        cursor = new_cursor;

        cmd = KeyCommand::None;

        shared_buffer.lock().unwrap().clear();

        let mut foo = rows.clone();
        shared_buffer.lock().unwrap().append(&mut foo);

        send_back.send(true);
    }
}

fn main() {
    let mut glfw = glfw::init(glfw::FAIL_ON_ERRORS).unwrap();
    glfw.window_hint(glfw::WindowHint::ContextVersion(3, 3));
    glfw.window_hint(glfw::WindowHint::OpenGlProfile(
        glfw::OpenGlProfileHint::Core,
    ));
    #[cfg(target_os = "macos")]
    glfw.window_hint(glfw::WindowHint::OpenGlForwardCompat(true));

    let (mut window, events) = glfw
        .create_window(300, 300, "Hello this is window", glfw::WindowMode::Windowed)
        .expect("Failed to create GLFW window.");

    window.set_all_polling(true);
    window.make_current();

    gl::load_with(|symbol| window.get_proc_address(symbol) as *const _);

    let (send, recv): (Sender<KeyCommand>, Receiver<KeyCommand>) = channel();

    let (send_back, recv_back): (Sender<bool>, Receiver<bool>) = channel();

    let layout_task = Builder::new().name("layout task".to_string());

    let buffer: Arc<Mutex<Vec<Buf>>> = Arc::new(Mutex::new(vec![]));

    let clone_buffer = buffer.clone();

    let font = "./Lato-Regular.ttf";
    let mut font_measure: HashMap<char, FontSize> = create_font_map(&font);

    let layout_task_done = layout_task.spawn(move || {
        text_model(recv, clone_buffer, send_back, font_measure);
    });

    let (shader_program, vao, mut some_fn) = opengl::setup(300.0, 300.0);

    while !window.should_close() {
        glfw.poll_events();

        match recv_back.try_recv() {
            Ok(_) => {
                let read_buffer_result = buffer.lock().unwrap();
                some_fn(read_buffer_result.len());
            }
            _ => (),
        }

        unsafe {
            gl::ClearColor(0.2, 0.3, 0.3, 1.0);
            gl::Clear(gl::COLOR_BUFFER_BIT);

            gl::UseProgram(shader_program);
            gl::BindVertexArray(vao);
            gl::DrawElements(gl::TRIANGLES, 100, gl::UNSIGNED_INT, std::ptr::null());
        }

        //let read_buffer_result = buffer.lock().unwrap();

        for (_, event) in glfw::flush_messages(&events) {
            handle_window_event(&mut window, event, &send);
        }

        window.swap_buffers();
        glfw.poll_events();
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
