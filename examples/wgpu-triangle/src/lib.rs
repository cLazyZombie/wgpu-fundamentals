use std::cell::RefCell;
use std::rc::Rc;

use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, console};

struct State {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    canvas_id: String,
    size: (u32, u32),
}

impl State {
    async fn new(canvas_id: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let canvas = get_canvas(canvas_id).map_err(|e| format!("Failed to get canvas: {:?}", e))?;
        let size = get_canvas_size(&canvas);

        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());

        let surface = instance
            .create_surface(wgpu::SurfaceTarget::Canvas(canvas.clone()))
            .unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .unwrap();

        let (device, queue) = adapter
            .request_device(&wgpu::DeviceDescriptor {
                label: Some("main device"),
                required_features: wgpu::Features::default(),
                required_limits: wgpu::Limits::default(),
                ..Default::default()
            })
            .await
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.0,
            height: size.1,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        surface.configure(&device, &surface_config);

        // 셰이더 생성
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        // 렌더 파이프라인 생성
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render Pipeline Layout"),
                bind_group_layouts: &[],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs_main"),
                buffers: &[],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        Ok(Self {
            device,
            queue,
            surface,
            surface_config,
            render_pipeline,
            canvas_id: canvas_id.to_string(),
            size,
        })
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.draw(0..3, 0..1);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }

    fn resize(&mut self, new_size: (u32, u32)) {
        let limits = wgpu::Limits::default();
        let new_size = (
            new_size.0.max(1).min(limits.max_texture_dimension_2d),
            new_size.1.max(1).min(limits.max_texture_dimension_2d),
        );

        if new_size == self.size {
            return;
        }

        self.size = new_size;
        self.surface_config.width = new_size.0;
        self.surface_config.height = new_size.1;
        self.surface.configure(&self.device, &self.surface_config);
    }
}

fn start_render_loop(state: Rc<RefCell<State>>) {
    let f = Rc::new(RefCell::new(None));
    let g = Rc::clone(&f);

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        // try_borrow_mut을 사용하여 panic 방지
        match state.try_borrow_mut() {
            Ok(mut state) => {
                // Resize canvas if necessary
                let canvas =
                    get_canvas(&state.canvas_id).expect("Failed to get canvas for resizing");
                let (width, height) = get_canvas_size(&canvas);
                if (width, height) != state.size {
                    state.resize((width, height));
                    console::log_1(&format!("Resized to: {}x{}", width, height).into());
                }

                match state.render() {
                    Ok(_) => {}
                    Err(wgpu::SurfaceError::Lost) => {
                        state
                            .surface
                            .configure(&state.device, &state.surface_config);
                        console::log_1(&"Surface lost, reconfiguring".into());
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        console::log_1(&"Out of memory!".into());
                        return; // 렌더 루프 중단
                    }
                    Err(e) => {
                        console::log_1(&format!("Render error: {:?}", e).into());
                    }
                }
            }
            Err(_) => {
                // State가 다른 곳에서 빌려져 있음 - 이번 프레임 스킵
                console::log_1(&"State borrowed elsewhere, skipping frame".into());
            }
        }

        // Schedule next frame
        request_animation_frame(f.borrow().as_ref().unwrap());
    }) as Box<dyn FnMut()>));

    request_animation_frame(g.borrow().as_ref().unwrap());
}

fn request_animation_frame(f: &Closure<dyn FnMut()>) {
    web_sys::window()
        .unwrap()
        .request_animation_frame(f.as_ref().unchecked_ref())
        .expect("Failed to request animation frame");
}

fn get_canvas(canvas_id: &str) -> Result<HtmlCanvasElement, JsValue> {
    let window = web_sys::window().ok_or_else(|| JsValue::from_str("Failed to get window"))?;
    let document = window
        .document()
        .ok_or_else(|| JsValue::from_str("Failed to get document"))?;

    document
        .get_element_by_id(canvas_id)
        .ok_or_else(|| JsValue::from_str(&format!("Canvas with id '{}' not found", canvas_id)))?
        .dyn_into::<HtmlCanvasElement>()
        .map_err(|_| JsValue::from_str("Element is not a canvas"))
}

fn get_canvas_size(canvas: &HtmlCanvasElement) -> (u32, u32) {
    let device_pixel_ratio = web_sys::window().unwrap().device_pixel_ratio();
    let client_rect = canvas.get_bounding_client_rect();
    (
        (client_rect.width() * device_pixel_ratio) as u32,
        (client_rect.height() * device_pixel_ratio) as u32,
    )
}

#[wasm_bindgen]
pub async fn run(canvas_id: &str) -> Result<(), JsValue> {
    console_error_panic_hook::set_once();

    let state = Rc::new(RefCell::new(State::new(canvas_id).await.unwrap()));
    start_render_loop(state);
    Ok(())
}
