#![allow(dead_code)]
#![allow(non_snake_case)]
#![allow(unreachable_code)]
#![allow(unused_mut)]
#![allow(unused_unsafe)]
#![allow(unused_assignments)]
#![allow(unused_variables)]
extern crate nalgebra_glm as glm;
use std::str::FromStr;
use std::{ mem, ptr, os::raw::c_void };
use std::thread;
use std::sync::{Mutex, Arc, RwLock};
use std::fmt::Debug;

mod shader;
mod util;

mod scene_graph;
use glm::normalize;
use scene_graph::SceneNode;

mod toolbox;

mod mesh;
use mesh::{Mesh, Helicopter};

use glutin::event::{Event, WindowEvent, DeviceEvent, KeyboardInput, ElementState::{Pressed, Released}, VirtualKeyCode::{self, *}};
use glutin::event_loop::ControlFlow;
use shader::Shader;

// initial window size
const INITIAL_SCREEN_W: u32 = 800;
const INITIAL_SCREEN_H: u32 = 600;

// == // Helper functions to make interacting with OpenGL a little bit prettier. You *WILL* need these! // == //

// Get the size of an arbitrary array of numbers measured in bytes
// Example usage:  pointer_to_array(my_array)
fn byte_size_of_array<T>(val: &[T]) -> isize {
    std::mem::size_of_val(&val[..]) as isize
}

// Get the OpenGL-compatible pointer to an arbitrary array of numbers
// Example usage:  pointer_to_array(my_array)
fn pointer_to_array<T>(val: &[T]) -> *const c_void {
    &val[0] as *const T as *const c_void
}

// Get the size of the given type in bytes
// Example usage:  size_of::<u64>()
fn size_of<T>() -> i32 {
    mem::size_of::<T>() as i32
}

// Get an offset in bytes for n units of type T, represented as a relative pointer
// Example usage:  offset::<u64>(4)
fn offset<T>(n: u32) -> *const c_void {
    (n * mem::size_of::<T>() as u32) as *const T as *const c_void
}

unsafe fn create_vao(vertices: &Vec<f32>, indices: &Vec<u32>, colors: &Vec<f32>,normals: &Vec<f32>) -> u32 {

    let mut VAO:u32 = 0;
    gl::GenVertexArrays(1, &mut VAO);
    gl::BindVertexArray(VAO);
    
    let mut VBO:u32 = 0;
    gl::GenBuffers(1, &mut VBO);
    gl::BindBuffer(gl::ARRAY_BUFFER, VBO);
    gl::BufferData(
        gl::ARRAY_BUFFER, 
        byte_size_of_array(vertices), 
        pointer_to_array(vertices), 
        gl::STATIC_DRAW,
    );

    gl::VertexAttribPointer(
        0, 
        3, 
        gl::FLOAT, 
        gl::FALSE, 
        0,
        ptr::null()
    );
    gl::EnableVertexAttribArray(0);

    let mut CBO:u32 = 0;
    gl::GenBuffers(1, &mut CBO);
    gl::BindBuffer(gl::ARRAY_BUFFER, CBO);
    gl::BufferData(
        gl::ARRAY_BUFFER, 
        byte_size_of_array(colors), 
        pointer_to_array(colors), 
        gl::STATIC_DRAW,
    );


    gl::VertexAttribPointer(
        1, 
        4, 
        gl::FLOAT, 
        gl::FALSE, 
        4 * size_of::<f32>(),
        ptr::null()
    );
    gl::EnableVertexAttribArray(1);

    let mut NBO:u32 = 0;
    gl::GenBuffers(1, &mut NBO);
    gl::BindBuffer(gl::ARRAY_BUFFER, NBO);
    gl::BufferData(
        gl::ARRAY_BUFFER, 
        byte_size_of_array(normals), 
        pointer_to_array(normals), 
        gl::STATIC_DRAW,
    );


    gl::VertexAttribPointer(
        2, 
        3, 
        gl::FLOAT, 
        gl::FALSE, 
        3 * size_of::<f32>(),
        ptr::null()
    );
    gl::EnableVertexAttribArray(2);

    let mut IBO:u32 = 0;
    gl::GenBuffers(1, &mut IBO);
    gl::BindBuffer(gl::ELEMENT_ARRAY_BUFFER, IBO);
    gl::BufferData(
        gl::ELEMENT_ARRAY_BUFFER, 
        byte_size_of_array(indices),
        pointer_to_array(indices),
        gl::STATIC_DRAW 
    );



    VAO
}

fn get_values_from_str<T>(filename:&str)-> Vec<T> 
where T: std::str::FromStr, <T as FromStr>::Err: Debug
{
    let vertices:Vec<T> = filename
        .split(',')
        .filter(|&i| !i.trim().is_empty())
        .map(|i| i.trim().parse::<T>().unwrap())
        .collect();
    vertices
}

fn main() {
    // Set up the necessary objects to deal with windows and event handling
    let el = glutin::event_loop::EventLoop::new();
    let wb = glutin::window::WindowBuilder::new()
        .with_title("Gloom-rs")
        .with_resizable(true)
        .with_inner_size(glutin::dpi::LogicalSize::new(INITIAL_SCREEN_W, INITIAL_SCREEN_H));
    let cb = glutin::ContextBuilder::new()
        .with_vsync(true);
    let windowed_context = cb.build_windowed(wb, &el).unwrap();
    // Uncomment these if you want to use the mouse for controls, but want it to be confined to the screen and/or invisible.
    // windowed_context.window().set_cursor_grab(true).expect("failed to grab cursor");
    // windowed_context.window().set_cursor_visible(false);

    // Set up a shared vector for keeping track of currently pressed keys
    let arc_pressed_keys = Arc::new(Mutex::new(Vec::<VirtualKeyCode>::with_capacity(10)));
    // Make a reference of this vector to send to the render thread
    let pressed_keys = Arc::clone(&arc_pressed_keys);

    // Set up shared tuple for tracking mouse movement between frames
    let arc_mouse_delta = Arc::new(Mutex::new((0f32, 0f32)));
    // Make a reference of this tuple to send to the render thread
    let mouse_delta = Arc::clone(&arc_mouse_delta);

    // Set up shared tuple for tracking changes to the window size
    let arc_window_size = Arc::new(Mutex::new((INITIAL_SCREEN_W, INITIAL_SCREEN_H, false)));
    // Make a reference of this tuple to send to the render thread
    let window_size = Arc::clone(&arc_window_size);

    // Spawn a separate thread for rendering, so event handling doesn't block rendering
    let render_thread = thread::spawn(move || {
        // Acquire the OpenGL Context and load the function pointers.
        // This has to be done inside of the rendering thread, because
        // an active OpenGL context cannot safely traverse a thread boundary
        let context = unsafe {
            let c = windowed_context.make_current().unwrap();
            gl::load_with(|symbol| c.get_proc_address(symbol) as *const _);
            c
        };

        let mut window_aspect_ratio = INITIAL_SCREEN_W as f32 / INITIAL_SCREEN_H as f32;

        // Set up openGL
        unsafe {
            gl::Enable(gl::DEPTH_TEST);
            gl::DepthFunc(gl::LESS);
            gl::Enable(gl::CULL_FACE);
            gl::Disable(gl::MULTISAMPLE);
            gl::Enable(gl::BLEND);
            gl::BlendFunc(gl::SRC_ALPHA, gl::ONE_MINUS_SRC_ALPHA);
            gl::Enable(gl::DEBUG_OUTPUT_SYNCHRONOUS);
            gl::DebugMessageCallback(Some(util::debug_callback), ptr::null());

            // Print some diagnostics
            println!("{}: {}", util::get_gl_string(gl::VENDOR), util::get_gl_string(gl::RENDERER));
            println!("OpenGL\t: {}", util::get_gl_string(gl::VERSION));
            println!("GLSL\t: {}", util::get_gl_string(gl::SHADING_LANGUAGE_VERSION));
        }
        let terrain:Mesh = mesh::Terrain::load("resources/lunarsurface.obj");
        let heli:Helicopter = mesh::Helicopter::load("resources/helicopter.obj");

        let surface_vao = unsafe { create_vao(&terrain.vertices,&terrain.indices, &terrain.colors, &terrain.normals) };
        
        let h_body_vao = unsafe { create_vao(&heli.body.vertices,&heli.body.indices, &heli.body.colors, &heli.body.normals) };
        let h_door_vao = unsafe { create_vao(&heli.door.vertices,&heli.door.indices, &heli.door.colors, &heli.door.normals) };
        let h_main_rotor_vao = unsafe { create_vao(&heli.main_rotor.vertices,&heli.main_rotor.indices, &heli.main_rotor.colors, &heli.main_rotor.normals) };
        let h_tail_rotor_vao = unsafe { create_vao(&heli.tail_rotor.vertices,&heli.tail_rotor.indices, &heli.tail_rotor.colors, &heli.tail_rotor.normals) };

        let mut root = SceneNode::new();
        
        let mut surface = SceneNode::from_vao(surface_vao, terrain.index_count);
        root.add_child(&surface);

        let mut helis:Vec<scene_graph::Node> = vec![];
        let mut main_rotors:Vec<scene_graph::Node> = vec![];
        let mut tail_rotors:Vec<scene_graph::Node> = vec![];
        for i in 0..5 {
            let mut h_body = SceneNode::from_vao(h_body_vao, heli.body.index_count); 
            let mut h_door = SceneNode::from_vao(h_door_vao, heli.door.index_count); 
            let mut h_main_rotor = SceneNode::from_vao(h_main_rotor_vao, heli.main_rotor.index_count); 
            let mut h_tail_rotor = SceneNode::from_vao(h_tail_rotor_vao, heli.tail_rotor.index_count); 
            h_tail_rotor.reference_point = glm::vec3(0.35, 2.3, 10.4);
            h_main_rotor.reference_point = glm::vec3(0., 1., 0.);
            h_body.reference_point = glm::vec3(0.1,0.1, 0.1);
            h_door.reference_point = glm::vec3(0., 0., 0.);
            
            h_body.position = glm::vec3(0., i as f32 *10., 0.);

            surface.add_child(&h_body);
            h_body.add_child(&h_door);
            h_body.add_child(&h_main_rotor);
            h_body.add_child(&h_tail_rotor);

            helis.push(h_body);
            main_rotors.push(h_main_rotor);
            tail_rotors.push(h_tail_rotor);
        }

    

        root.print();
        println!("Press [T] to toggle free moving camera");
        let simple_shader: Shader = unsafe {
            shader::ShaderBuilder::new()
            .attach_file("shaders/simple.vert")
            .attach_file("shaders/simple.frag")
            .link()
        };
        unsafe {
            simple_shader.activate()
        }

        // Used to demonstrate keyboard handling for exercise 2.
        let mut _arbitrary_number = 0.0; // feel free to remove


        // The main rendering loop
        let first_frame_time = std::time::Instant::now();
        let mut prevous_frame_time = first_frame_time;

        let cname = std::ffi::CString::new("x").expect("CString::new failed");

        let mut x:f32 = 70.;
        let mut y:f32 = -40.;
        let mut z:f32 = 70.;

        let mut yaw:f32 = 103.;
        let mut pitch:f32 = 18.;

        let mut toggle_camera = true;
        let mut pause_heli = true;
        loop {
            // Compute time passed since the previous frame and since the start of the program
            let now = std::time::Instant::now();
            let elapsed = now.duration_since(first_frame_time).as_secs_f32();
            let delta_time = now.duration_since(prevous_frame_time).as_secs_f32();
            prevous_frame_time = now;

            // Handle resize events
            if let Ok(mut new_size) = window_size.lock() {
                if new_size.2 {
                    context.resize(glutin::dpi::PhysicalSize::new(new_size.0, new_size.1));
                    window_aspect_ratio = new_size.0 as f32 / new_size.1 as f32;
                    (*new_size).2 = false;
                    println!("Resized");
                    unsafe { gl::Viewport(0, 0, new_size.0 as i32, new_size.1 as i32); }
                }
            }

            // Handle keyboard input
            if let Ok(keys) = pressed_keys.lock() {
                let move_sens = delta_time*100.;
                let rot_sens = delta_time*60.;
                for key in keys.iter() {
                    match key {
                        // The `VirtualKeyCode` enum is defined here:
                        //    https://docs.rs/winit/0.25.0/winit/event/enum.VirtualKeyCode.html
                        VirtualKeyCode::A => {
                            x += move_sens;
                        }
                        VirtualKeyCode::D => {
                            x -= move_sens;
                        }
                        VirtualKeyCode::W => {
                            z += move_sens;
                        }
                        VirtualKeyCode::S => {
                            z -= move_sens;
                        }
                        VirtualKeyCode::LShift => {
                            y += move_sens;
                        }
                        VirtualKeyCode::Space => {
                            y -= move_sens;
                        }
                        VirtualKeyCode::Left => {
                            yaw -= rot_sens;
                        }
                        VirtualKeyCode::Right => {
                            yaw += rot_sens;
                        }
                        VirtualKeyCode::Up => {
                            pitch -= rot_sens;
                        }
                        VirtualKeyCode::Down => {
                            pitch += rot_sens;
                        }
                        VirtualKeyCode::T => {
                            toggle_camera = !toggle_camera;
                        }
                        VirtualKeyCode::H => {
                            pause_heli = !pause_heli;
                        }
                        // default handler:
                        _ => { }
                    }
                }
                //println!("x: {x} y: {y} z: {z} - yaw: {yaw} pitch: {pitch}");
                //context.window().set_title(format!("x: {x:.2} y: {y:.2} z: {z:.2} - yaw: {yaw:.2} pitch: {pitch:.2}").as_str());
                
            }
            
            // Handle mouse movement. delta contains the x and y movement of the mouse since last frame in pixels
            if let Ok(mut delta) = mouse_delta.lock() {

                // == // Optionally access the acumulated mouse movement between
                // == // frames here with `delta.0` and `delta.1`

                *delta = (0.0, 0.0); // reset when done
            }

            // == // Please compute camera transforms here (exercise 2 & 3)
            
            let mut trans_matrix: glm::Mat4 = glm::identity();
            let heading = toolbox::simple_heading_animation(elapsed);

            let ct:glm::Mat4;
            if toggle_camera {
                ct = glm::translation(&glm::vec3(70.-heading.x,y,-heading.z));
                x = heading.x;
                z = heading.z;
            } else {
                ct = glm::translation(&glm::vec3(x,y,z));
            }
            let cyaw:glm::Mat4 = glm::rotation(yaw.to_radians(), &glm::vec3(0., 1., 0.));
            let cpitch:glm::Mat4 = glm::rotation(pitch.to_radians(), &glm::vec3(1., 0., 0.));

            trans_matrix = ct       * trans_matrix;
            trans_matrix = cyaw     * trans_matrix;
            trans_matrix = cpitch   * trans_matrix;
            

            let perspective_mat: glm::Mat4 = glm::perspective(1., 1., 1., 10000.);       
            trans_matrix = perspective_mat * trans_matrix;

            unsafe {
                // Clear the color and depth buffers
                gl::ClearColor(0.035, 0.046, 0.078, 1.0); // night sky, full opacity
                gl::Clear(gl::COLOR_BUFFER_BIT | gl::DEPTH_BUFFER_BIT);

                gl::UniformMatrix4fv(3, 1, gl::FALSE, (trans_matrix).as_ptr());

                for i in 0..5 {
                    helis[i].rotation = glm::vec3(heading.pitch,heading.yaw,heading.roll);
                    tail_rotors[i].rotation = glm::vec3(elapsed*20., 0., 0.);
                    main_rotors[i].rotation = glm::vec3(0., elapsed*20., 0.);
                    if pause_heli {
                        helis[i].position = glm::vec3(heading.x+i as f32 *20., helis[i].position[1], heading.z+ i as f32 *20.);
                    }    
                }


                // == // Issue the necessary gl:: commands to draw your scene here
                draw_scene(&root, &trans_matrix, &glm::identity());
            }

            // Display the new color buffer on the display
            context.swap_buffers().unwrap(); // we use "double buffering" to avoid artifacts
        }
    });


    // == //
    // == // From here on down there are only internals.
    // == //


    // Keep track of the health of the rendering thread
    let render_thread_healthy = Arc::new(RwLock::new(true));
    let render_thread_watchdog = Arc::clone(&render_thread_healthy);
    thread::spawn(move || {
        if !render_thread.join().is_ok() {
            if let Ok(mut health) = render_thread_watchdog.write() {
                println!("Render thread panicked!");
                *health = false;
            }
        }
    });

    // Start the event loop -- This is where window events are initially handled
    el.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        // Terminate program if render thread panics
        if let Ok(health) = render_thread_healthy.read() {
            if *health == false {
                *control_flow = ControlFlow::Exit;
            }
        }

        match event {
            Event::WindowEvent { event: WindowEvent::Resized(physical_size), .. } => {
                println!("New window size! width: {}, height: {}", physical_size.width, physical_size.height);
                if let Ok(mut new_size) = arc_window_size.lock() {
                    *new_size = (physical_size.width, physical_size.height, true);
                }
            }
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                *control_flow = ControlFlow::Exit;
            }
            // Keep track of currently pressed keys to send to the rendering thread
            Event::WindowEvent { event: WindowEvent::KeyboardInput {
                    input: KeyboardInput { state: key_state, virtual_keycode: Some(keycode), .. }, .. }, .. } => {

                if let Ok(mut keys) = arc_pressed_keys.lock() {
                    match key_state {
                        Released => {
                            if keys.contains(&keycode) {
                                let i = keys.iter().position(|&k| k == keycode).unwrap();
                                keys.remove(i);
                            }
                        },
                        Pressed => {
                            if !keys.contains(&keycode) {
                                keys.push(keycode);
                            }
                        }
                    }
                }

                // Handle Escape and Q keys separately
                match keycode {
                    Escape => { *control_flow = ControlFlow::Exit; }
                    Q      => { *control_flow = ControlFlow::Exit; }
                    _      => { }
                }
            }
            Event::DeviceEvent { event: DeviceEvent::MouseMotion { delta }, .. } => {
                // Accumulate mouse movement
                if let Ok(mut position) = arc_mouse_delta.lock() {
                    *position = (position.0 + delta.0 as f32, position.1 + delta.1 as f32);
                }
            }
            _ => { }
        }
    });
}


unsafe fn draw_scene(node: &scene_graph::SceneNode,
    view_projection_matrix: &glm::Mat4,
    transformation_so_far: &glm::Mat4) {
        
    let mut node_trans:glm::Mat4 = glm::identity();
        
    if node.index_count != -1 {
        // Rotate around rererence point
        node_trans = glm::translation(&glm::vec3(node.reference_point[0]*-1., node.reference_point[1]*-1., node.reference_point[2]*-1.)) *  node_trans;
        node_trans = glm::rotation(node.rotation[2], &glm::vec3(0.,0., node.reference_point[2])) *  node_trans; 
        node_trans = glm::rotation(node.rotation[1], &glm::vec3(0.,node.reference_point[1],0.))  *  node_trans; 
        node_trans = glm::rotation(node.rotation[0], &glm::vec3(node.reference_point[0],0., 0.)) *  node_trans; 
        node_trans = glm::translation(&node.reference_point) *  node_trans;

        node_trans = glm::translation(&node.position) * node_trans;

        node_trans = transformation_so_far * node_trans;


        // Check if node is drawable, if so: set uniforms and draw
        gl::UniformMatrix4fv(3, 1, gl::FALSE, (view_projection_matrix*node_trans).as_ptr());
        let mut node_3x3:glm::Mat3 = glm::mat4_to_mat3(&node_trans);

        gl::UniformMatrix3fv(4, 1, gl::FALSE, node_3x3.as_ptr());
        gl::BindVertexArray(node.vao_id);
        gl::DrawElements(gl::TRIANGLES, node.index_count,gl::UNSIGNED_INT,ptr::null());
    }
    // Recurse
    for &child in &node.children {
        draw_scene(&*child, &view_projection_matrix, &node_trans);
    }
    }
    