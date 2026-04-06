// Selection shader - GPU-accelerated instanced quad rendering for text selection
// Replaces per-character rect_filled() calls with single instanced draw

struct Uniforms {
  screen_size: vec2<f32>,
  selection_color: vec4<f32>,
}

struct SelectionRect {
  rect_min: vec2<f32>,
  rect_max: vec2<f32>,
}

struct VertexOut {
  @builtin(position) position: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

@group(0) @binding(1)
var<storage, read> instances: array<SelectionRect>;

@vertex
fn vs_main(
  @builtin(vertex_index) v_idx: u32,
  @builtin(instance_index) i_idx: u32
) -> VertexOut {
  var out: VertexOut;

  let inst = instances[i_idx];

  // Generate quad vertices from vertex index (triangle strip: 0,1,2,3 -> TL,TR,BL,BR)
  // Bit 0: x select (0=min, 1=max)
  // Bit 1: y select (0=min, 1=max)
  let x = select(inst.rect_min.x, inst.rect_max.x, (v_idx & 1u) != 0u);
  let y = select(inst.rect_min.y, inst.rect_max.y, (v_idx & 2u) != 0u);

  // Convert screen coordinates to NDC
  // Screen: (0,0) top-left, (width,height) bottom-right
  // NDC: (-1,-1) bottom-left, (1,1) top-right
  let ndc_x = (x / uniforms.screen_size.x) * 2.0 - 1.0;
  let ndc_y = 1.0 - (y / uniforms.screen_size.y) * 2.0;

  out.position = vec4<f32>(ndc_x, ndc_y, 0.0, 1.0);
  return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
  return uniforms.selection_color;
}
