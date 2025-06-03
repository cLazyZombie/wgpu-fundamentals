use std::cell::RefCell;
use std::rc::Rc;
use wasm_bindgen::prelude::*;
use web_sys::{HtmlCanvasElement, ResizeObserver, ResizeObserverEntry, console};

thread_local! {
    static GLOBAL_STATE: RefCell<Option<Rc<RefCell<State>>>> = RefCell::new(None);
}

struct State {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    render_pipeline: wgpu::RenderPipeline,
    size: (u32, u32),
}

impl State {
    async fn new(canvas: HtmlCanvasElement) -> Result<Self, Box<dyn std::error::Error>> {
        // get canvas pixel size
        let device_pixel_ratio = web_sys::window().unwrap().device_pixel_ratio();
        let client_rect = canvas.get_bounding_client_rect();
        let size = (
            (client_rect.width() * device_pixel_ratio) as u32,
            (client_rect.height() * device_pixel_ratio) as u32,
        );

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
        self.size = new_size;
        self.surface_config.width = new_size.0;
        self.surface_config.height = new_size.1;
        self.surface.configure(&self.device, &self.surface_config);
    }
}

#[wasm_bindgen]
pub fn resize_callback(entries: js_sys::Array, _observer: ResizeObserver) {
    for i in 0..entries.length() {
        let entry = entries.get(i);
        let entry: ResizeObserverEntry = entry.dyn_into().unwrap();

        let device_pixel_ratio = web_sys::window().unwrap().device_pixel_ratio();
        let content_rect = entry.content_rect();
        let new_size = (
            (content_rect.width() * device_pixel_ratio) as u32,
            (content_rect.height() * device_pixel_ratio) as u32,
        );

        GLOBAL_STATE.with(|global| {
            if let Some(state_rc) = &*global.borrow() {
                let mut state = state_rc.borrow_mut();
                if state.size != new_size {
                    state.resize(new_size);
                }
            }
        });

        console::log_1(
            &format!(
                "[resize_callback] Content Rect - x: {}, y: {}, width: {}, height: {}, width_in_px: {}, height_in_px: {}",
                content_rect.x(),
                content_rect.y(),
                content_rect.width(),
                content_rect.height(),
                content_rect.width() * device_pixel_ratio,
                content_rect.height() * device_pixel_ratio,
            )
            .into(),
        );
    }
}

fn start_render_loop(state: Rc<RefCell<State>>) {
    let f = Rc::new(RefCell::new(None));
    let g = Rc::clone(&f);

    *g.borrow_mut() = Some(Closure::wrap(Box::new(move || {
        {
            let mut state = state.borrow_mut();

            match state.render() {
                Ok(_) => {}
                Err(wgpu::SurfaceError::Lost) => {
                    state
                        .surface
                        .configure(&state.device, &state.surface_config);
                    console::log_1(&"surface lost.".into());
                }
                Err(wgpu::SurfaceError::OutOfMemory) => {
                    console::log_1(&"Out of memory!".into());
                }
                Err(e) => {
                    console::log_1(&format!("Render error: {:?}", e).into());
                }
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

#[wasm_bindgen]
pub async fn run(canvas_id: &str) {
    console_error_panic_hook::set_once();

    let window = web_sys::window().unwrap();
    let document = window.document().unwrap();
    let canvas = document
        .get_element_by_id(canvas_id)
        .unwrap()
        .dyn_into::<HtmlCanvasElement>()
        .unwrap();

    let state = Rc::new(RefCell::new(State::new(canvas.clone()).await.unwrap()));

    GLOBAL_STATE.with(|global| {
        *global.borrow_mut() = Some(state.clone());
    });

    // reigster resize observer
    {
        // ResizeObserver 콜백 함수를 JavaScript 함수로 변환
        let callback =
            Closure::wrap(Box::new(resize_callback) as Box<dyn Fn(js_sys::Array, ResizeObserver)>);

        let observer = ResizeObserver::new(callback.as_ref().unchecked_ref()).unwrap();
        observer.observe(&canvas);

        // 콜백을 메모리에서 해제되지 않도록 유지
        callback.forget();
    }

    start_render_loop(state);
}
