//! Animated wave background component - GPU accelerated
//!
//! This module implements the exact OpenXMB wave algorithm using WGPU shaders.
//!
//! Based on RetroArch's pipeline_ribbon.vert shader used in PS3 XMB interface.
//! See: https://github.com/libretro/RetroArch/blob/master/gfx/drivers/vulkan_shaders/pipeline_ribbon.vert

use crate::assets::theme::get_theme;

use codelord_core::animation::resources::ContinuousAnimations;
use codelord_core::ecs::world::World;
use codelord_core::theme::ThemeAnimation;

use eframe::{egui, egui_wgpu};
use egui_wgpu::wgpu;

/// Render the animated wave background using GPU acceleration.
/// Uses ECS ContinuousAnimations to track animation state.
pub fn show(ui: &mut egui::Ui, rect: egui::Rect, time: f32, world: &mut World) {
  let line_color = get_line_color(world);

  // Create the callback for WGPU rendering
  let callback = egui_wgpu::Callback::new_paint_callback(
    rect,
    WavePaintCallback {
      time,
      screen_width: rect.width(),
      screen_height: rect.height(),
      line_color,
    },
  );

  ui.painter().add(callback);

  if let Some(mut cont) = world.get_resource_mut::<ContinuousAnimations>() {
    cont.set_wave_active();
  }
}

/// Get wave line color (animated during theme transitions)
fn get_line_color(world: &World) -> [f32; 4] {
  if let Some(anim) = world.get_resource::<ThemeAnimation>()
    && !anim.is_complete
  {
    let c = &anim.current_colors.primary;
    return [c.r, c.g, c.b, c.a];
  }

  let [r, g, b, a] = get_theme(world).primary;
  [
    r as f32 / 255.0,
    g as f32 / 255.0,
    b as f32 / 255.0,
    a as f32 / 255.0,
  ]
}

/// Paint callback for rendering the wave background
struct WavePaintCallback {
  time: f32,
  screen_width: f32,
  screen_height: f32,
  line_color: [f32; 4],
}

impl egui_wgpu::CallbackTrait for WavePaintCallback {
  fn prepare(
    &self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    _screen_descriptor: &egui_wgpu::ScreenDescriptor,
    egui_encoder: &mut wgpu::CommandEncoder,
    resources: &mut egui_wgpu::CallbackResources,
  ) -> Vec<wgpu::CommandBuffer> {
    let resources: &mut WaveRenderResources = resources.get_mut().unwrap();
    resources.prepare(
      device,
      queue,
      egui_encoder,
      self.time,
      self.screen_width,
      self.screen_height,
      self.line_color,
    );
    Vec::new()
  }

  fn paint(
    &self,
    _info: egui::PaintCallbackInfo,
    render_pass: &mut wgpu::RenderPass<'static>,
    resources: &egui_wgpu::CallbackResources,
  ) {
    let resources: &WaveRenderResources = resources.get().unwrap();
    resources.paint(render_pass, &_info);
  }
}

/// Uniforms for the wave shader
#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct WaveUniforms {
  time: f32,
  screen_width: f32,
  screen_height: f32,
  layer_count: f32,
  line_color: [f32; 4],
}

/// Resources for rendering the wave background
struct WaveRenderResources {
  wave_pipeline: wgpu::RenderPipeline,
  wave_bind_group: wgpu::BindGroup,
  uniform_buffer: wgpu::Buffer,
  blit_pipeline: wgpu::RenderPipeline,
  blit_bind_group: wgpu::BindGroup,
  blit_bind_group_layout: wgpu::BindGroupLayout,
  msaa_texture: wgpu::Texture,
  msaa_view: wgpu::TextureView,
  resolve_texture: wgpu::Texture,
  resolve_view: wgpu::TextureView,
  sampler: wgpu::Sampler,
  sample_count: u32,
  target_format: wgpu::TextureFormat,
  current_width: u32,
  current_height: u32,
}

impl WaveRenderResources {
  fn new(device: &wgpu::Device, target_format: wgpu::TextureFormat) -> Self {
    let sample_count = 4; // 4x MSAA
    // Start with reasonable default size - will resize dynamically
    let width = 800;
    let height = 600;

    // Create MSAA texture
    let msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
      label: Some("wave_msaa_texture"),
      size: wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count,
      dimension: wgpu::TextureDimension::D2,
      format: target_format,
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      view_formats: &[],
    });
    let msaa_view =
      msaa_texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Create resolve texture
    let resolve_texture = device.create_texture(&wgpu::TextureDescriptor {
      label: Some("wave_resolve_texture"),
      size: wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: target_format,
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT
        | wgpu::TextureUsages::TEXTURE_BINDING,
      view_formats: &[],
    });
    let resolve_view =
      resolve_texture.create_view(&wgpu::TextureViewDescriptor::default());

    // Create sampler for blit
    let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
      label: Some("wave_sampler"),
      address_mode_u: wgpu::AddressMode::ClampToEdge,
      address_mode_v: wgpu::AddressMode::ClampToEdge,
      address_mode_w: wgpu::AddressMode::ClampToEdge,
      mag_filter: wgpu::FilterMode::Linear,
      min_filter: wgpu::FilterMode::Linear,
      ..Default::default()
    });

    // Load wave shader
    let wave_shader =
      device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("wave_shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
          include_str!("../../../../codelord-assets/shaders/wave_shader.wgsl"),
        )),
      });

    // Load blit shader
    let blit_shader =
      device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("blit_shader"),
        source: wgpu::ShaderSource::Wgsl(std::borrow::Cow::Borrowed(
          include_str!("../../../../codelord-assets/shaders/blit_shader.wgsl"),
        )),
      });

    // === WAVE PIPELINE SETUP ===
    let wave_bind_group_layout =
      device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("wave_bind_group_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
          binding: 0,
          visibility: wgpu::ShaderStages::VERTEX,
          ty: wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Uniform,
            has_dynamic_offset: false,
            min_binding_size: None,
          },
          count: None,
        }],
      });

    let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
      label: Some("wave_uniform_buffer"),
      size: std::mem::size_of::<WaveUniforms>() as u64,
      usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
      mapped_at_creation: false,
    });

    let wave_bind_group =
      device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("wave_bind_group"),
        layout: &wave_bind_group_layout,
        entries: &[wgpu::BindGroupEntry {
          binding: 0,
          resource: uniform_buffer.as_entire_binding(),
        }],
      });

    let wave_pipeline_layout =
      device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("wave_pipeline_layout"),
        bind_group_layouts: &[&wave_bind_group_layout],
        push_constant_ranges: &[],
      });

    let wave_pipeline =
      device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("wave_pipeline"),
        layout: Some(&wave_pipeline_layout),
        vertex: wgpu::VertexState {
          module: &wave_shader,
          entry_point: Some("vs_main"),
          buffers: &[],
          compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
          module: &wave_shader,
          entry_point: Some("fs_main"),
          targets: &[Some(wgpu::ColorTargetState {
            format: target_format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
          })],
          compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
          topology: wgpu::PrimitiveTopology::TriangleStrip,
          ..Default::default()
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
          count: sample_count,
          mask: !0,
          alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
      });

    // === BLIT PIPELINE SETUP ===
    let blit_bind_group_layout =
      device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("blit_bind_group_layout"),
        entries: &[
          wgpu::BindGroupLayoutEntry {
            binding: 0,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Texture {
              sample_type: wgpu::TextureSampleType::Float { filterable: true },
              view_dimension: wgpu::TextureViewDimension::D2,
              multisampled: false,
            },
            count: None,
          },
          wgpu::BindGroupLayoutEntry {
            binding: 1,
            visibility: wgpu::ShaderStages::FRAGMENT,
            ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
            count: None,
          },
        ],
      });

    let blit_bind_group =
      device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("blit_bind_group"),
        layout: &blit_bind_group_layout,
        entries: &[
          wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&resolve_view),
          },
          wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::Sampler(&sampler),
          },
        ],
      });

    let blit_pipeline_layout =
      device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("blit_pipeline_layout"),
        bind_group_layouts: &[&blit_bind_group_layout],
        push_constant_ranges: &[],
      });

    let blit_pipeline =
      device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("blit_pipeline"),
        layout: Some(&blit_pipeline_layout),
        vertex: wgpu::VertexState {
          module: &blit_shader,
          entry_point: Some("vs_main"),
          buffers: &[],
          compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
          module: &blit_shader,
          entry_point: Some("fs_main"),
          targets: &[Some(wgpu::ColorTargetState {
            format: target_format,
            blend: Some(wgpu::BlendState::ALPHA_BLENDING),
            write_mask: wgpu::ColorWrites::ALL,
          })],
          compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState::default(),
        depth_stencil: None,
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        cache: None,
      });

    Self {
      wave_pipeline,
      wave_bind_group,
      uniform_buffer,
      blit_pipeline,
      blit_bind_group,
      blit_bind_group_layout,
      msaa_texture,
      msaa_view,
      resolve_texture,
      resolve_view,
      sampler,
      sample_count,
      target_format,
      current_width: width,
      current_height: height,
    }
  }

  /// Resize textures if needed. Returns true if resize occurred.
  fn resize_if_needed(
    &mut self,
    device: &wgpu::Device,
    width: u32,
    height: u32,
  ) {
    // Add some padding to avoid frequent resizes, and clamp to reasonable
    // bounds
    let target_width = width.clamp(256, 2560);
    let target_height = height.clamp(256, 1440);

    // Only resize if significantly different (>20% change or too small)
    let width_ratio = target_width as f32 / self.current_width.max(1) as f32;
    let height_ratio = target_height as f32 / self.current_height.max(1) as f32;

    let needs_resize = !(0.5..=1.2).contains(&width_ratio)
      || !(0.5..=1.2).contains(&height_ratio);

    if !needs_resize {
      return;
    }

    // Recreate MSAA texture
    self.msaa_texture = device.create_texture(&wgpu::TextureDescriptor {
      label: Some("wave_msaa_texture"),
      size: wgpu::Extent3d {
        width: target_width,
        height: target_height,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: self.sample_count,
      dimension: wgpu::TextureDimension::D2,
      format: self.target_format,
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
      view_formats: &[],
    });
    self.msaa_view = self
      .msaa_texture
      .create_view(&wgpu::TextureViewDescriptor::default());

    // Recreate resolve texture
    self.resolve_texture = device.create_texture(&wgpu::TextureDescriptor {
      label: Some("wave_resolve_texture"),
      size: wgpu::Extent3d {
        width: target_width,
        height: target_height,
        depth_or_array_layers: 1,
      },
      mip_level_count: 1,
      sample_count: 1,
      dimension: wgpu::TextureDimension::D2,
      format: self.target_format,
      usage: wgpu::TextureUsages::RENDER_ATTACHMENT
        | wgpu::TextureUsages::TEXTURE_BINDING,
      view_formats: &[],
    });
    self.resolve_view = self
      .resolve_texture
      .create_view(&wgpu::TextureViewDescriptor::default());

    // Recreate bind group with new texture view
    self.blit_bind_group =
      device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("blit_bind_group"),
        layout: &self.blit_bind_group_layout,
        entries: &[
          wgpu::BindGroupEntry {
            binding: 0,
            resource: wgpu::BindingResource::TextureView(&self.resolve_view),
          },
          wgpu::BindGroupEntry {
            binding: 1,
            resource: wgpu::BindingResource::Sampler(&self.sampler),
          },
        ],
      });

    self.current_width = target_width;
    self.current_height = target_height;
  }

  #[allow(clippy::too_many_arguments)]
  fn prepare(
    &mut self,
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    encoder: &mut wgpu::CommandEncoder,
    time: f32,
    screen_width: f32,
    screen_height: f32,
    line_color: [f32; 4],
  ) {
    // Dynamic texture resizing based on actual widget size
    self.resize_if_needed(device, screen_width as u32, screen_height as u32);

    let uniforms = WaveUniforms {
      time,
      screen_width,
      screen_height,
      layer_count: 5.0, // 5 layers for depth
      line_color,
    };

    queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

    // Render wave to MSAA texture with automatic resolve
    {
      let mut render_pass =
        encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
          label: Some("wave_msaa_pass"),
          color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: &self.msaa_view,
            resolve_target: Some(&self.resolve_view),
            ops: wgpu::Operations {
              load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
              store: wgpu::StoreOp::Store,
            },
            depth_slice: None,
          })],
          depth_stencil_attachment: None,
          timestamp_writes: None,
          occlusion_query_set: None,
        });

      render_pass.set_pipeline(&self.wave_pipeline);
      render_pass.set_bind_group(0, &self.wave_bind_group, &[]);

      // Draw 5 layers × 256 segments × 2 vertices per segment
      let layer_count = 5;
      let segments_per_layer = 256;
      let vertices_per_layer = (segments_per_layer + 1) * 2;
      let total_vertices = layer_count * vertices_per_layer;

      render_pass.draw(0..total_vertices, 0..1);
    }
  }

  fn paint(
    &self,
    render_pass: &mut wgpu::RenderPass<'static>,
    _info: &egui::PaintCallbackInfo,
  ) {
    // Blit the MSAA-resolved wave texture to egui's render pass
    // Note: We don't set viewport or scissor rect here - egui handles the
    // clipping through the rect passed to new_paint_callback. Setting
    // viewport/scissor here causes incorrect rendering during page
    // transitions.
    render_pass.set_pipeline(&self.blit_pipeline);
    render_pass.set_bind_group(0, &self.blit_bind_group, &[]);
    render_pass.draw(0..3, 0..1); // Fullscreen triangle
  }
}

/// Callback trait implementation for egui_wgpu
pub struct WaveCallback;

impl WaveCallback {
  /// Initialize the WGPU resources for the wave background
  pub fn init(cc: &eframe::CreationContext<'_>) {
    if let Some(render_state) = cc.wgpu_render_state.as_ref() {
      let device = &render_state.device;
      let target_format = render_state.target_format;

      render_state
        .renderer
        .write()
        .callback_resources
        .insert(WaveRenderResources::new(device, target_format));
    }
  }
}
