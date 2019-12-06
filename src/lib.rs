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
    varying highp vec3 vLighting;

    void main(void) {
        gl_FragColor = vec4(vec3(1, 1, 1) * vLighting, 1);
    }
"#;

static VERTEX_SHADER: &'static str = r#"
    attribute vec4 aVertexPosition;
    attribute vec3 aVertexNormal;

    uniform mat4 uNormalMatrix;
    uniform mat4 uModelViewMatrix;
    uniform mat4 uProjectionMatrix;

    varying highp vec3 vLighting;

    void main(void) {
        gl_Position = uProjectionMatrix * uModelViewMatrix * aVertexPosition;
        // Apply lighting effect
        highp vec3 ambientLight = vec3(0.3, 0.3, 0.3);
        highp vec3 directionalLightColor = vec3(1, 1, 1);
        highp vec3 directionalVector = normalize(vec3(0.85, 0.8, 0.75));
        highp vec4 transformedNormal = uNormalMatrix * vec4(aVertexNormal, 1.0);
        highp float directional = max(dot(transformedNormal.xyz, directionalVector), 0.0);
        vLighting = ambientLight + (directionalLightColor * directional);
    }
"#;

#[wasm_bindgen]
pub fn start() -> Result<(), JsValue> {
    let context = get_webgl_context_by_id("canvas");

    let shader_program = init_shaders(&context);

    {
        let f = Rc::new(RefCell::new(None));
        let g = f.clone();
        *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {

            request_animation_frame(f.borrow().as_ref().unwrap());
        }) as Box<dyn FnMut()>));

        request_animation_frame(g.borrow().as_ref().unwrap());
    }

    Ok(())
}

fn get_webgl_context_by_id(id: &str) -> WebGlRenderingContext {
    let document = web_sys::window()
        .unwrap()
        .document()
        .unwrap();

    let canvas = document
        .get_element_by_id(id)
        .unwrap()
        .dyn_into::<web_sys::HtmlCanvasElement>()
        .unwrap();

    let context = canvas
        .get_context("webgl")
        .unwrap()
        .unwrap()
        .dyn_into::<WebGlRenderingContext>()
        .unwrap();

    context.viewport(0, 0, canvas.width() as i32, canvas.height() as i32);

    context
}

fn get_shader(context: &WebGlRenderingContext, shader_type: u32, source: &str) -> WebGlShader {
    let shader = context.create_shader(shader_type).unwrap();

    context.shader_source(&shader, source);
    context.compile_shader(&shader);
    let compile_is_succeeded = context.get_shader_parameter(&shader, WebGlRenderingContext::COMPILE_STATUS).as_bool().unwrap();
    if !compile_is_succeeded {
        panic!("シェーダーのコンパイルでエラーが発生しました");
    }
    shader
}

fn init_shaders(context: &WebGlRenderingContext) -> WebGlProgram {
    let fragment_shader = get_shader(&context, WebGlRenderingContext::FRAGMENT_SHADER, FRAGMENT_SHADER);
    let vertex_shader = get_shader(&context, WebGlRenderingContext::VERTEX_SHADER, VERTEX_SHADER);

    let shader_program = context.create_program().unwrap();
    context.attach_shader(&shader_program, &vertex_shader);
    context.attach_shader(&shader_program, &fragment_shader);
    context.link_program(&shader_program);

    let shader_is_created = context.get_program_parameter(&shader_program, WebGlRenderingContext::LINK_STATUS).as_bool().unwrap();

    if !shader_is_created {
        let info = context.get_program_info_log(&shader_program).unwrap();
        error(&format!("シェーダープログラムを初期化できません: {}", info));
    }

    context.use_program(Some(&shader_program));

    let vertex_position_attribute = context.get_attrib_location(&shader_program, "aVertexPosition");
    context.enable_vertex_attrib_array(vertex_position_attribute as u32);

    shader_program
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

