/*use std::sync::mpsc::*;
use std::thread::Builder;

fn main() {
    let (send, recv): (Sender<String>, Receiver<String>) = channel();

    let layout_task = Builder::new().name("layout task".to_string());

    let layout_task_done = layout_task.spawn(move || loop {

        let value = recv.recv();
        println!("{:?}",value);

    });

    loop {

        send.send("Hello world".to_string());


    }
}*/
extern crate glfw;

use glfw::{Action, Context, Key};
use num_traits::{clamp, clamp_max, clamp_min, sign};

use std::sync::mpsc::*;
use std::thread::Builder;

#[derive(Debug)]
enum KeyCommand {
    Left,
    Right,
    Back,
    Value(String),
    NewLine,
    Size(i32, i32),
}

fn text_model(recv: Receiver<KeyCommand>) {
    let mut rows: Vec<String> = vec![String::with_capacity(3)];
    let mut cursor_x = 0;
    let mut cursor_y = 0;

    loop {
        let value = recv.recv();

        match value.unwrap() {
            KeyCommand::Value(string) => {
                let mut get_line = &mut rows[cursor_y];
                get_line.insert_str(cursor_x, &string);

                cursor_x += 1;
            }
            KeyCommand::NewLine => {
                cursor_y += 1;
                cursor_x = 0;
                rows.push(String::with_capacity(3));
            }
            KeyCommand::Left => {
                if cursor_x > 0 {
                    cursor_x -= 1;
                }
            }
            KeyCommand::Right => {
                if cursor_x < 10 {
                    cursor_x += 1;
                }
            }
            KeyCommand::Back => {
                if cursor_x > 0 {
                    let mut get_line = &mut rows[cursor_y];
                    get_line.remove(cursor_x - 1);
                    cursor_x -= 1;
                }
            }
            KeyCommand::Size(width, height) => {
               println!("{:?} {:?}", width, height); 
            }
            _ => (),
        }

        println!("{:?}", rows);
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
            sender.send(KeyCommand::Size(w,h));
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
