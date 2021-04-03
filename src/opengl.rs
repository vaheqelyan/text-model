extern crate nalgebra_glm as glm;
const RGB_FACTOR: f32 = 1.0 / 255.0;
use num_traits::{clamp_max, clamp_min};

extern crate gl;
use self::gl::types::*;

use std::ffi::CString;
use std::mem;
use std::os::raw::c_void;
use std::ptr;
use std::str;

use cgmath::Matrix;
use core::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

const VERTEX_SOURCE: &str = r#"
            #version 430 core
            layout(location = 0) in vec2 position;
            uniform mat4 MVP;

            void main()
            {
                gl_Position = MVP * vec4(position.x, position.y, 0.0, 1.0);
            }
            "#;

const FRAGMENT_SOURCE: &str = r#"
#version 430 core
out vec4 frag_color;

void main() {
  frag_color = vec4(0.0, 0.0, 0.0, 1.0);
}
"#;

pub fn create_shader(vertex_shader_source: &str, fragment_shader_source: &str) -> u32 {
    // build and compile our shader program
    // ------------------------------------
    // vertex shader
    unsafe {
        let vertex_shader = gl::CreateShader(gl::VERTEX_SHADER);
        let c_str_vert = CString::new(vertex_shader_source.as_bytes()).unwrap();
        gl::ShaderSource(vertex_shader, 1, &c_str_vert.as_ptr(), ptr::null());
        gl::CompileShader(vertex_shader);

        // check for shader compile errors
        let mut success = gl::FALSE as GLint;
        let mut info_log = Vec::with_capacity(512);
        info_log.set_len(512 - 1); // subtract 1 to skip the trailing null character
        gl::GetShaderiv(vertex_shader, gl::COMPILE_STATUS, &mut success);
        if success != gl::TRUE as GLint {
            gl::GetShaderInfoLog(
                vertex_shader,
                512,
                ptr::null_mut(),
                info_log.as_mut_ptr() as *mut GLchar,
            );
            println!(
                "ERROR::SHADER::VERTEX::COMPILATION_FAILED\n{}",
                str::from_utf8(&info_log).unwrap()
            );
        }

        // fragment shader
        let fragment_shader = gl::CreateShader(gl::FRAGMENT_SHADER);
        let c_str_frag = CString::new(fragment_shader_source.as_bytes()).unwrap();
        gl::ShaderSource(fragment_shader, 1, &c_str_frag.as_ptr(), ptr::null());
        gl::CompileShader(fragment_shader);
        // check for shader compile errors
        gl::GetShaderiv(fragment_shader, gl::COMPILE_STATUS, &mut success);
        if success != gl::TRUE as GLint {
            gl::GetShaderInfoLog(
                fragment_shader,
                512,
                ptr::null_mut(),
                info_log.as_mut_ptr() as *mut GLchar,
            );
            println!(
                "ERROR::SHADER::FRAGMENT::COMPILATION_FAILED\n{}",
                str::from_utf8(&info_log).unwrap()
            );
        }

        // link shaders
        let shader_program = gl::CreateProgram();
        gl::AttachShader(shader_program, vertex_shader);
        gl::AttachShader(shader_program, fragment_shader);
        gl::LinkProgram(shader_program);
        // check for linking errors
        gl::GetProgramiv(shader_program, gl::LINK_STATUS, &mut success);
        if success != gl::TRUE as GLint {
            gl::GetProgramInfoLog(
                shader_program,
                512,
                ptr::null_mut(),
                info_log.as_mut_ptr() as *mut GLchar,
            );
            println!(
                "ERROR::SHADER::PROGRAM::COMPILATION_FAILED\n{}",
                str::from_utf8(&info_log).unwrap()
            );
        }
        gl::DeleteShader(vertex_shader);
        gl::DeleteShader(fragment_shader);

        shader_program
    }
}

pub fn make_index(elements_count: u32) -> Vec<u32> {
    let mut n: u32 = 0;

    let mut x: Vec<u32> = Vec::with_capacity((elements_count as usize) * 7);

    for _ in 0..elements_count {
        x.append(&mut vec![n, n + 1, n + 2, n + 2, n + 3, n, 0xFFFF]);

        n = n + 4;
    }

    x
}

pub fn setup(w: f32, h: f32) -> (u32, u32, impl FnMut(usize)) {
    unsafe {
        let shader_program = create_shader(VERTEX_SOURCE, FRAGMENT_SOURCE);

        /*let mut vertex_buffer: Vec<f32> = vec![
            10.0, 10.0, //0
            50.0, 10.0, //1
            50.0, 50.0, //2
            10.0, 50.0,
        ];*/

        let mut vertex_cleaner: Vec<f32> = Vec::with_capacity(800);
        let mut vertex_buffer: Vec<f32> = Vec::with_capacity(800);

        (0..100).into_iter().for_each(|_| {
            vertex_cleaner.append(&mut vec![
                0.0, 0.0, //0
                0.0, 0.0, //1
                0.0, 0.0, //2
                0.0, 0.0,
            ]);
        });

        vertex_buffer.append(&mut vertex_cleaner);

        let (mut vbo, mut vao, mut ebo) = (0, 0, 0);
        gl::GenVertexArrays(1, &mut vao);
        gl::GenBuffers(1, &mut vbo);
        gl::GenBuffers(1, &mut ebo);
        gl::BindVertexArray(vao);

        let map_flags = gl::MAP_WRITE_BIT | gl::MAP_PERSISTENT_BIT | gl::MAP_COHERENT_BIT;
        let create_flags = map_flags | gl::DYNAMIC_STORAGE_BIT;

        gl::BindBuffer(gl::ARRAY_BUFFER, vbo);
        gl::BufferStorage(
            gl::ARRAY_BUFFER,
            (vertex_buffer.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            //&0f32 as *const f32 as *const c_void,
            &vertex_buffer[0] as *const f32 as *const c_void,
            create_flags,
        );

        let index_buffer = make_index(800);
        //let index_buffer: Vec<u32> = vec![0, 1, 2, 2, 3, 0];

        gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, ebo);
        gl::BufferStorage(
            gl::ELEMENT_ARRAY_BUFFER,
            (index_buffer.len() * mem::size_of::<GLfloat>()) as GLsizeiptr,
            //&0u32 as *const u32 as *const c_void,
            &index_buffer[0] as *const u32 as *const c_void,
            create_flags,
        );

        let pointer = gl::MapBufferRange(gl::ARRAY_BUFFER, 0, 800, map_flags);
        let save_pointer = Some(pointer);

        let stripe = 2 * mem::size_of::<GLfloat>() as GLsizei;
        gl::VertexAttribPointer(0, 2, gl::FLOAT, gl::FALSE, stripe, ptr::null());
        gl::EnableVertexAttribArray(0);

        gl::BindBuffer(gl::ARRAY_BUFFER, 0);

        gl::BindVertexArray(0);

        gl::UseProgram(shader_program);

        gl::Enable(gl::PRIMITIVE_RESTART);
        gl::PrimitiveRestartIndex(0xFFFF);

        let c_str_vert = CString::new("MVP".as_bytes()).unwrap();

        let model_loc = gl::GetUniformLocation(shader_program, c_str_vert.as_ptr());

        let model = cgmath::ortho(0.0, w, h, 0.0, -1.0, 1.0);

        gl::UniformMatrix4fv(model_loc, 1, gl::FALSE, model.as_ptr());

        let some_fn = move |len| {
            (0..len).into_iter().enumerate().for_each(|(pos, _)| {
                let index = (pos + 1) as f32;

                let new = vec![
                    10.0 + index,
                    10.0 + index, //0
                    50.0 + index,
                    10.0 + index, //1
                    50.0 + index,
                    50.0 + index, //2
                    10.0 + index,
                    50.0 + index,
                ];

                let start = 8 * pos;
                let end = 8 * (pos + 1);

                vertex_buffer.splice(start..end, new.iter().cloned());
            });

            for x in len * 8..vertex_buffer.len() {
                vertex_buffer[x] = 0.0;
            }

            ptr::copy_nonoverlapping(
                vertex_buffer.as_ptr(),
                save_pointer.unwrap() as *mut f32,
                vertex_buffer.len(),
            );
        };

        (shader_program, vao, some_fn)
    }
}
