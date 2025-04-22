// tests/simple_box_render.rs
use wgpu::util::DeviceExt;
use winit::{event::*, event_loop::{ControlFlow, EventLoop}, window::WindowBuilder};

const VERTICES: &[f32] = &[
    // positions         // colors
    -0.5, -0.5,  0.5,    1.0, 0.0, 0.0,
     0.5, -0.5,  0.5,    0.0, 1.0, 0.0,
     0.5,  0.5,  0.5,    0.0, 0.0, 1.0,
    -0.5,  0.5,  0.5,    1.0, 1.0, 0.0,

    -0.5, -0.5, -0.5,    1.0, 0.0, 1.0,
     0.5, -0.5, -0.5,    0.0, 1.0, 1.0,
     0.5,  0.5, -0.5,    1.0, 1.0, 1.0,
    -0.5,  0.5, -0.5,    0.0, 0.0, 0.0,
];

const INDICES: &[u16] = &[
    0, 1, 2,  2, 3, 0,
    4, 5, 6,  6, 7, 4,
    0, 1, 5,  5, 4, 0,
    2, 3, 7,  7, 6, 2,
    1, 2, 6,  6, 5, 1,
    0, 3, 7,  7, 4, 0,
];

pub async fn run() {
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().with_title("3D Box Test").build(&event_loop).unwrap();

    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::default());
    let surface = unsafe { instance.create_surface(&window) }.unwrap();
    let adapter = instance.request_adapter(&wgpu::RequestAdapterOptions {
        power_preference: wgpu::PowerPreference::HighPerformance,
        compatible_surface: Some(&surface),
        force_fallback_adapter: false,
    }).await.unwrap();

    let (device, queue) = adapter.request_device(&Default::default(), None).await.unwrap();
    let config = surface.get_default_config(&adapter, 800, 600).unwrap();
    surface.configure(&device, &config);

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Vertex Buffer"),
        contents: bytemuck::cast_slice(VERTICES),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let index_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("Index Buffer"),
        contents: bytemuck::cast_slice(INDICES),
        usage: wgpu::BufferUsages::INDEX,
    });

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("Box Shader"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../simple_box.wgsl").into()),
    });

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("Pipeline Layout"),
        bind_group_layouts: &[],
        push_constant_ranges: &[],
    });

    let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(&pipeline_layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[wgpu::VertexBufferLayout {
                array_stride: 6 * 4,
                step_mode: wgpu::VertexStepMode::Vertex,
                attributes: &[
                    wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 0, shader_location: 0 },
                    wgpu::VertexAttribute { format: wgpu::VertexFormat::Float32x3, offset: 3 * 4, shader_location: 1 },
                ],
            }],
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(config.format.into())],
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
    });

    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Poll;
        match event {
            Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => *control_flow = ControlFlow::Exit,
            Event::RedrawRequested(_) => {
                let frame = surface.get_current_texture().unwrap();
                let view = frame.texture.create_view(&Default::default());
                let mut encoder = device.create_command_encoder(&Default::default());

                {
                    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                        label: Some("Render Pass"),
                        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                            view: &view,
                            resolve_target: None,
                            ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::BLACK), store: true },
                        })],
                        depth_stencil_attachment: None,
                    });

                    render_pass.set_pipeline(&render_pipeline);
                    render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
                    render_pass.set_index_buffer(index_buffer.slice(..), wgpu::IndexFormat::Uint16);
                    render_pass.draw_indexed(0..INDICES.len() as u32, 0, 0..1);
                }

                queue.submit(Some(encoder.finish()));
                frame.present();
            }
            Event::MainEventsCleared => window.request_redraw(),
            _ => {}
        }
    });
}

fn main() { pollster::block_on(run()); }