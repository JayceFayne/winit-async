use anyhow::{Error, bail};
use std::f64;
use std::sync::Arc;
use vello::kurbo::{Affine, BezPath, Circle, Ellipse, Line, RoundedRect, Stroke};
use vello::wgpu::util::TextureBlitter;
use vello::wgpu::{
    BackendOptions, Backends, CommandEncoderDescriptor, CompositeAlphaMode, CurrentSurfaceTexture,
    Device, DeviceDescriptor, ExperimentalFeatures, Extent3d, Features, Instance,
    InstanceDescriptor, InstanceFlags, Limits, MemoryHints, PowerPreference, PresentMode, Queue,
    RequestAdapterOptions, Surface, SurfaceColorSpace, SurfaceConfiguration, Texture,
    TextureDescriptor, TextureDimension, TextureFormat, TextureUsages, TextureView,
    TextureViewDescriptor, Trace,
};
use vello::{AaConfig, AaSupport, RenderParams, Renderer, RendererOptions, Scene, peniko::Color};
use winit_async::winit::dpi::{LogicalSize, PhysicalSize};
use winit_async::winit::{event::WindowEvent, window::Window};
use winit_async::{WindowExtAsync, create_window, resumed, run_app};

fn create_vello_texture(device: &Device, width: u32, height: u32) -> (Texture, TextureView) {
    let texture = device.create_texture(&TextureDescriptor {
        label: Some("vello-intermediate-texture"),
        size: Extent3d {
            width: width.max(1),
            height: height.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: TextureDimension::D2,
        format: TextureFormat::Rgba8Unorm,
        usage: TextureUsages::STORAGE_BINDING | TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });

    let view = texture.create_view(&TextureViewDescriptor::default());

    (texture, view)
}

struct RenderState {
    window: Arc<Window>,
    surface: Surface<'static>,
    device: Device,
    queue: Queue,
    renderer: Renderer,
    surface_config: SurfaceConfiguration,
    vello_texture: Texture,
    vello_view: TextureView,
    blitter: TextureBlitter,
    size: PhysicalSize<u32>,
    scene: Scene,
}

impl RenderState {
    async fn new(window: Arc<Window>) -> Result<Self, anyhow::Error> {
        let size = window.inner_size();
        let instance = Instance::new(InstanceDescriptor {
            backends: Backends::all(),
            flags: InstanceFlags::default(),
            memory_budget_thresholds: Default::default(),
            backend_options: BackendOptions::default(),
            display: None,
        });
        let surface = instance.create_surface(window.clone())?;

        let adapter = instance
            .request_adapter(&RequestAdapterOptions {
                power_preference: PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
                apply_limit_buckets: false,
            })
            .await?;

        let (device, queue) = adapter
            .request_device(&DeviceDescriptor {
                label: Some("vello-components-device"),
                required_features: Features::empty(),
                required_limits: Limits::default(),
                memory_hints: MemoryHints::Performance,
                trace: Trace::Off,
                experimental_features: ExperimentalFeatures::disabled(),
            })
            .await?;

        let capabilities = surface.get_capabilities(&adapter);

        if capabilities.formats.is_empty() {
            bail!("The surface reports no supported texture formats.");
        }

        let format = capabilities
            .formats
            .iter()
            .copied()
            .find(|format| {
                matches!(
                    format,
                    TextureFormat::Bgra8UnormSrgb | TextureFormat::Rgba8UnormSrgb
                )
            })
            .or_else(|| {
                capabilities.formats.iter().copied().find(|format| {
                    matches!(
                        format,
                        TextureFormat::Bgra8Unorm | TextureFormat::Rgba8Unorm
                    )
                })
            })
            .unwrap_or(capabilities.formats[0]);

        let present_mode = if capabilities.present_modes.contains(&PresentMode::Fifo) {
            PresentMode::Fifo
        } else {
            capabilities.present_modes[0]
        };

        let alpha_mode = capabilities
            .alpha_modes
            .first()
            .copied()
            .unwrap_or(CompositeAlphaMode::Auto);

        let surface_config = SurfaceConfiguration {
            usage: TextureUsages::RENDER_ATTACHMENT,
            format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
            color_space: SurfaceColorSpace::Auto,
        };

        surface.configure(&device, &surface_config);

        let renderer = Renderer::new(
            &device,
            RendererOptions {
                use_cpu: false,
                antialiasing_support: AaSupport::all(),
                num_init_threads: None,
                pipeline_cache: None,
            },
        )?;

        let (vello_texture, vello_view) =
            create_vello_texture(&device, size.width.max(1), size.height.max(1));

        let blitter = TextureBlitter::new(&device, surface_config.format);

        Ok(Self {
            window,
            surface,
            device,
            queue,
            renderer,
            surface_config,
            vello_texture,
            vello_view,
            blitter,
            size,
            scene: Scene::new(),
        })
    }

    fn resize(&mut self, size: PhysicalSize<u32>) {
        if size.width == 0 || size.height == 0 {
            return;
        }

        self.size = size;
        self.surface_config.width = size.width;
        self.surface_config.height = size.height;
        self.surface.configure(&self.device, &self.surface_config);
        let (texture, view) = create_vello_texture(&self.device, size.width, size.height);
        self.vello_texture = texture;
        self.vello_view = view;
    }

    fn render(&mut self) -> Result<(), Error> {
        add_shapes_to_scene(&mut self.scene);

        self.renderer.render_to_texture(
            &self.device,
            &self.queue,
            &self.scene,
            &self.vello_view,
            &RenderParams {
                base_color: Color::WHITE,
                width: self.size.width,
                height: self.size.height,
                antialiasing_method: AaConfig::Area,
            },
        )?;
        self.scene.reset();
        let frame = match self.surface.get_current_texture() {
            CurrentSurfaceTexture::Success(surface_texture) => surface_texture,
            CurrentSurfaceTexture::Outdated | CurrentSurfaceTexture::Suboptimal(_) => {
                self.surface.configure(&self.device, &self.surface_config);
                self.window.request_redraw();
                return Ok(());
            }
            CurrentSurfaceTexture::Occluded | CurrentSurfaceTexture::Timeout => {
                self.window.request_redraw();
                return Ok(());
            }
            CurrentSurfaceTexture::Lost => bail!("Surface was lost"),
            CurrentSurfaceTexture::Validation => {
                bail!("Validation error getting surface")
            }
        };

        let surface_view = frame.texture.create_view(&TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&CommandEncoderDescriptor {
                label: Some("vello-surface-blit"),
            });

        self.blitter
            .copy(&self.device, &mut encoder, &self.vello_view, &surface_view);

        self.queue.submit(std::iter::once(encoder.finish()));
        self.window.pre_present_notify();
        self.queue.present(frame);

        Ok(())
    }
}

fn main() -> Result<(), winit_async::Error<Error>> {
    run_app(async {
        resumed().await;
        loop {
            let window = Arc::new(create_window(
                Window::default_attributes()
                    .with_title("Vello Shapes")
                    .with_inner_size(LogicalSize::new(1044, 800)),
            )?);

            let mut state = RenderState::new(window.clone()).await?;

            let mut window_events = window.events().unwrap();

            while let Some(event) = window_events.next_event().await {
                match event {
                    WindowEvent::CloseRequested => break,
                    WindowEvent::Resized(size) => {
                        state.resize(size);
                        state.window.request_redraw();
                    }
                    WindowEvent::RedrawRequested => {
                        state.render()?;
                    }
                    _ => {}
                }
            }
        }
    })
}
fn add_shapes_to_scene(scene: &mut Scene) {
    let stroke = Stroke::new(6.0);

    let rounded_rect = RoundedRect::new(10.0, 10.0, 240.0, 240.0, 20.0);
    scene.fill(
        vello::peniko::Fill::NonZero,
        Affine::IDENTITY,
        Color::new([0.12, 0.08, 0.22, 1.0]),
        None,
        &rounded_rect,
    );
    scene.stroke(
        &stroke,
        Affine::IDENTITY,
        Color::new([0.98, 0.35, 0.58, 1.0]),
        None,
        &rounded_rect,
    );

    let circle = Circle::new((420.0, 180.0), 120.0);
    scene.fill(
        vello::peniko::Fill::NonZero,
        Affine::IDENTITY,
        Color::new([0.08, 0.72, 0.82, 1.0]),
        None,
        &circle,
    );
    scene.stroke(
        &Stroke::new(10.0),
        Affine::IDENTITY,
        Color::new([0.95, 0.95, 1.0, 1.0]),
        None,
        &circle,
    );

    let ellipse = Ellipse::new((250.0, 420.0), (100.0, 160.0), -90.0);
    scene.fill(
        vello::peniko::Fill::NonZero,
        Affine::IDENTITY,
        Color::new([0.55, 0.22, 0.95, 1.0]),
        None,
        &ellipse,
    );
    scene.stroke(
        &Stroke::new(5.0),
        Affine::IDENTITY,
        Color::new([0.96, 0.55, 0.2, 1.0]),
        None,
        &ellipse,
    );

    let diamond = RoundedRect::new(360.0, 330.0, 540.0, 510.0, 28.0);
    scene.fill(
        vello::peniko::Fill::NonZero,
        Affine::rotate(f64::consts::FRAC_PI_4),
        Color::new([0.95, 0.18, 0.38, 1.0]),
        None,
        &diamond,
    );

    let inner_circle = Circle::new((500.0, 410.0), 55.0);
    scene.fill(
        vello::peniko::Fill::NonZero,
        Affine::IDENTITY,
        Color::new([1.0, 0.82, 0.2, 1.0]),
        None,
        &inner_circle,
    );

    let small_circle = Circle::new((105.0, 365.0), 48.0);
    scene.fill(
        vello::peniko::Fill::NonZero,
        Affine::IDENTITY,
        Color::new([0.15, 0.9, 0.55, 1.0]),
        None,
        &small_circle,
    );
    scene.stroke(
        &Stroke::new(8.0),
        Affine::IDENTITY,
        Color::new([0.05, 0.2, 0.15, 1.0]),
        None,
        &small_circle,
    );

    let triangle = BezPath::from_svg("M 430 40 L 520 170 L 350 170 Z").unwrap();
    scene.fill(
        vello::peniko::Fill::NonZero,
        Affine::IDENTITY,
        Color::new([1.0, 0.38, 0.08, 1.0]),
        None,
        &triangle,
    );
    scene.stroke(
        &Stroke::new(5.0),
        Affine::IDENTITY,
        Color::new([1.0, 0.9, 0.35, 1.0]),
        None,
        &triangle,
    );

    let star = BezPath::from_svg(
        "M 650 55 L 670 105 L 725 110 L 682 145 L 695 200 L 650 170 L 605 200 L 618 145 L 575 110 L 630 105 Z",
    )
    .unwrap();
    scene.fill(
        vello::peniko::Fill::NonZero,
        Affine::IDENTITY,
        Color::new([1.0, 0.75, 0.05, 1.0]),
        None,
        &star,
    );
    scene.stroke(
        &Stroke::new(5.0),
        Affine::IDENTITY,
        Color::new([0.95, 0.2, 0.45, 1.0]),
        None,
        &star,
    );

    let wave = BezPath::from_svg(
        "M 20 560 C 100 480, 160 640, 240 560 S 380 480, 460 560 S 600 640, 700 540",
    )
    .unwrap();
    scene.stroke(
        &Stroke::new(14.0),
        Affine::IDENTITY,
        Color::new([0.2, 0.75, 1.0, 1.0]),
        None,
        &wave,
    );

    let arc =
        BezPath::from_svg("M 520 300 C 560 240, 650 240, 690 300 C 650 270, 560 270, 520 300 Z")
            .unwrap();
    scene.fill(
        vello::peniko::Fill::NonZero,
        Affine::IDENTITY,
        Color::new([0.75, 0.2, 0.95, 1.0]),
        None,
        &arc,
    );

    let line = Line::new((260.0, 20.0), (620.0, 100.0));
    scene.stroke(
        &Stroke::new(9.0),
        Affine::IDENTITY,
        Color::new([0.25, 0.55, 1.0, 1.0]),
        None,
        &line,
    );

    let cross = BezPath::from_svg(
        "M 60 430 L 100 430 L 100 390 L 140 390 L 140 430 L 180 430 L 180 470 L 140 470 L 140 510 L 100 510 L 100 470 L 60 470 Z",
    )
    .unwrap();
    scene.fill(
        vello::peniko::Fill::NonZero,
        Affine::IDENTITY,
        Color::new([0.95, 0.25, 0.65, 1.0]),
        None,
        &cross,
    );
    scene.stroke(
        &Stroke::new(4.0),
        Affine::IDENTITY,
        Color::new([1.0, 0.85, 0.95, 1.0]),
        None,
        &cross,
    );

    let ring_outer = Circle::new((720.0, 390.0), 75.0);
    let ring_inner = Circle::new((720.0, 390.0), 42.0);

    scene.fill(
        vello::peniko::Fill::NonZero,
        Affine::IDENTITY,
        Color::new([0.98, 0.3, 0.12, 1.0]),
        None,
        &ring_outer,
    );
    scene.fill(
        vello::peniko::Fill::NonZero,
        Affine::IDENTITY,
        Color::new([0.08, 0.12, 0.28, 1.0]),
        None,
        &ring_inner,
    );

    let hexagon =
        BezPath::from_svg("M 610 450 L 670 415 L 730 450 L 730 520 L 670 555 L 610 520 Z").unwrap();
    scene.fill(
        vello::peniko::Fill::NonZero,
        Affine::IDENTITY,
        Color::new([0.1, 0.85, 0.65, 1.0]),
        None,
        &hexagon,
    );
    scene.stroke(
        &Stroke::new(7.0),
        Affine::IDENTITY,
        Color::new([0.03, 0.2, 0.25, 1.0]),
        None,
        &hexagon,
    );
}
