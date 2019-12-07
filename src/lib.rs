mod utils;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use web_sys::{WebGlRenderingContext, WebGlShader, WebGlBuffer, WebGlProgram, WebGlUniformLocation};
use std::rc::{Rc};
use std::cell::{RefCell};

extern crate nalgebra_glm as glm;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global
// allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    fn log(s: &str);

    #[wasm_bindgen(js_namespace = console)]
    fn error(s: &str);
}

static FRAGMENT_SHADER: &'static str = r#"
precision mediump float;
uniform float time;
uniform vec2 mouse;
uniform vec2 resolution;

void main(void){
    vec2 m = vec2(mouse.x * 2.0 - 1.0, -mouse.y * 2.0 + 1.0);
    vec2 p = (gl_FragCoord.xy * 2.0 - resolution) / min(resolution.x, resolution.y);
    float t = sin(length(m - p) * 30.0 + time * 5.0);
    gl_FragColor = vec4(vec3(t), 1.0);
}
"#;

static VERTEX_SHADER: &'static str = r#"
attribute vec3 position;

void main(void){
    gl_Position = vec4(position, 1.0);
}
"#;

#[wasm_bindgen]
pub fn start() -> Result<(), JsValue> {
    let canvas = get_canvas_element_by_id("canvas")?;
    let context = get_webgl_context(&canvas)?;

    let mouse_x = Rc::new(RefCell::new(0));
    let mouse_y = Rc::new(RefCell::new(0));
    let canvas_w = canvas.client_width();
    let canvas_h = canvas.client_height();

    {
        let mouse_x = mouse_x.clone();
        let mouse_y = mouse_y.clone();
        add_event_listener(&canvas, "mousemove", move |event| {
            let mouse_event = event.dyn_into::<web_sys::MouseEvent>().unwrap();
            *mouse_x.borrow_mut() = mouse_event.offset_x();
            *mouse_y.borrow_mut() = mouse_event.offset_y();
        })?;
    }

    let shader_program = match init_shaders(&context) {
        Ok(s) => s,
        Err(e) => return Err(e)
    };

    let ul_time = context.get_uniform_location(&shader_program, "time");
    let ul_mouse = context.get_uniform_location(&shader_program, "mouse");
    let ul_resolution = context.get_uniform_location(&shader_program, "resolution");

    let (position_buffer, index_buffer) = init_buffers(&context);
    let attrib_location = context.get_attrib_location(&shader_program, "position") as u32;

    context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&position_buffer));
    context.enable_vertex_attrib_array(attrib_location);
    context.vertex_attrib_pointer_with_i32(
        attrib_location,
        3,
        WebGlRenderingContext::FLOAT,
        false,
        0,
        0
    );
    context.bind_buffer(WebGlRenderingContext::ELEMENT_ARRAY_BUFFER, Some(&index_buffer));

    context.clear_color(0.0, 0.0, 0.0, 1.0);

    let start_time = get_current_time();

    start_animation(move || {
        context.clear(WebGlRenderingContext::COLOR_BUFFER_BIT);

        if let Some(ul_time) = &ul_time {
            let current_time = get_current_time();
            context.uniform1f(
                Some(&ul_time),
                (current_time - start_time) as f32
            );
        }

        if let Some(ul_mouse2) = &ul_mouse {
            context.uniform2fv_with_f32_array(
                Some(&ul_mouse2),
                &vec![*mouse_x.borrow() as f32 / canvas_w as f32, *mouse_y.borrow() as f32 / canvas_h as f32]
            );
        }

        if let Some(ul_resolution) = &ul_resolution {
            context.uniform2fv_with_f32_array(
                Some(&ul_resolution),
                &vec![canvas_w as f32, canvas_h as f32]
            );
        }

        context.draw_elements_with_i32(WebGlRenderingContext::TRIANGLES, 6, WebGlRenderingContext::UNSIGNED_SHORT, 0);
        context.flush();
    });

    Ok(())
}

fn get_canvas_element_by_id(id: &str) -> Result<web_sys::HtmlCanvasElement, JsValue> {
    let document = web_sys::window()
        .unwrap()
        .document()
        .unwrap();

    document.get_element_by_id(id)
        .ok_or(JsValue::from("Element doesn't exist."))?
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .or_else(|e| Err(JsValue::from(e)))
}

fn get_webgl_context(canvas: &web_sys::HtmlCanvasElement) -> Result<WebGlRenderingContext, JsValue> {
    let context = canvas
        .get_context("webgl")?
        .ok_or(JsValue::from("Couldn't get WebGL context.2"))?
        .dyn_into::<WebGlRenderingContext>()?;

    context.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);

    Ok(context)
}

fn get_shader(context: &WebGlRenderingContext, shader_type: u32, source: &str) -> Result<WebGlShader, JsValue> {
    let shader = context.create_shader(shader_type).unwrap();

    context.shader_source(&shader, source);
    context.compile_shader(&shader);
    let compile_is_succeeded = context.get_shader_parameter(&shader, WebGlRenderingContext::COMPILE_STATUS).as_bool().unwrap();
    if compile_is_succeeded {
        Ok(shader)
    } else {
        Err(JsValue::from(&context.get_shader_info_log(&shader).unwrap()))
    }
}

fn init_shaders(context: &WebGlRenderingContext) -> Result<WebGlProgram, JsValue> {
    let fragment_shader = get_shader(&context, WebGlRenderingContext::FRAGMENT_SHADER, FRAGMENT_SHADER)?;
    let vertex_shader = get_shader(&context, WebGlRenderingContext::VERTEX_SHADER, VERTEX_SHADER)?;

    let shader_program = context.create_program().unwrap();
    context.attach_shader(&shader_program, &vertex_shader);
    context.attach_shader(&shader_program, &fragment_shader);
    context.link_program(&shader_program);

    let shader_is_created = context.get_program_parameter(&shader_program, WebGlRenderingContext::LINK_STATUS).as_bool().unwrap();

    if !shader_is_created {
        let info = context.get_program_info_log(&shader_program).unwrap();
        return Err(JsValue::from(&format!("シェーダープログラムを初期化できません: {}", info)))
    }

    context.use_program(Some(&shader_program));

    Ok(shader_program)
}

fn init_buffers(context: &WebGlRenderingContext) -> (WebGlBuffer, WebGlBuffer) {
    let position = [
        -1.0,  1.0, 0.0,
         1.0,  1.0, 0.0,
        -1.0, -1.0, 0.0,
         1.0, -1.0, 0.0
    ];
    let position_buffer = context.create_buffer().unwrap();
    context.bind_buffer(WebGlRenderingContext::ARRAY_BUFFER, Some(&position_buffer));
    unsafe {
        context.buffer_data_with_array_buffer_view(
            WebGlRenderingContext::ARRAY_BUFFER,
            &js_sys::Float32Array::view(&position),
            WebGlRenderingContext::STATIC_DRAW
        );
    }


    let index = [
        0, 2, 1,
        1, 2, 3
    ];
    let index_buffer = context.create_buffer().unwrap();
    context.bind_buffer(WebGlRenderingContext::ELEMENT_ARRAY_BUFFER, Some(&index_buffer));
    unsafe {
        context.buffer_data_with_array_buffer_view(
            WebGlRenderingContext::ELEMENT_ARRAY_BUFFER,
            &js_sys::Uint16Array::view(&index),
            WebGlRenderingContext::STATIC_DRAW
        );
    }

    (position_buffer, index_buffer)
}

// fn format_as_matrix<T: std::fmt::Display>(vec: Vec<T>, len_row: usize, len_column: usize) -> String {
//     let len = vec.len();
//     if len != len_column * len_row {
//         panic!("vector couldn't be divided by len_row");
//     }

//     (0..len_row).into_iter().map(|i| {
//         (0..len_column).into_iter().map(|j| {
//             format!("{}", &vec[i*len_row+j])
//         }).collect::<Vec<_>>().join(",")
//     }).collect::<Vec<_>>().join("\n")
// }

fn get_current_time() -> f64 { // sec
    js_sys::Date::now() / 1000.0
}

fn window() -> web_sys::Window {
    web_sys::window().expect("no global `window` exists")
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    window()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("should register `requestAnimationFrame` OK");
}

fn add_event_listener<T>(target: &web_sys::Element, event_name: &str, handler: T) -> Result<(), JsValue>
where
    T: 'static + FnMut(web_sys::Event)
{
    let cb = Closure::wrap(Box::new(handler) as Box<dyn FnMut(_)>);
    target.add_event_listener_with_callback(event_name, cb.as_ref().unchecked_ref())?;
    cb.forget();

    Ok(())
}

fn start_animation<T>(mut handler: T)
where T: 'static + FnMut()
{ 
    let f = Rc::new(RefCell::new(None));
    let g = f.clone();
    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        handler();
        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    request_animation_frame(g.borrow().as_ref().unwrap());
}
