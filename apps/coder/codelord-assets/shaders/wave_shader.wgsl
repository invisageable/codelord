// Wave shader - Direct translation of OpenXMB algorithm to WGSL
// OPTIMIZED: Reduced redundant noise calls, pre-computed constants, inlined hash

struct Uniforms {
  time: f32,
  screen_width: f32,
  screen_height: f32,
  layer_count: f32,
  line_color: vec4<f32>,
}

struct VertexOut {
  @builtin(position) position: vec4<f32>,
  @location(0) color: vec4<f32>,
  @location(1) layer: f32,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

// OpenXMB's xmb_noise2 function - optimized with FMA pattern
fn xmb_noise2(x: f32, z: f32, time: f32) -> f32 {
  let z4 = z * 4.0;
  let phase = z + time * 0.1 + x;
  return cos(z4) * cos(phase);
}

// Optimized 3D noise - inlined hash computation, reduced temporaries
fn noise3_opt(p: vec3<f32>) -> f32 {
  let i = floor(p);
  let f = fract(p);

  // Smoothstep interpolation weights
  let u = f * f * (3.0 - 2.0 * f);

  // Pre-compute all 8 corner positions
  let i000 = i;
  let i100 = i + vec3(1.0, 0.0, 0.0);
  let i010 = i + vec3(0.0, 1.0, 0.0);
  let i110 = i + vec3(1.0, 1.0, 0.0);
  let i001 = i + vec3(0.0, 0.0, 1.0);
  let i101 = i + vec3(1.0, 0.0, 1.0);
  let i011 = i + vec3(0.0, 1.0, 1.0);
  let i111 = i + vec3(1.0, 1.0, 1.0);

  // Inline hash computation for all 8 corners
  // hash3(p) = fract((p3.x + p3.y) * p3.z) where p3 = fract(p * 0.1031) + dot(...)
  let k = 0.1031;
  let k2 = 33.33;

  var p3_000 = fract(i000 * k); p3_000 += dot(p3_000, p3_000.yzx + k2);
  let h000 = fract((p3_000.x + p3_000.y) * p3_000.z);

  var p3_100 = fract(i100 * k); p3_100 += dot(p3_100, p3_100.yzx + k2);
  let h100 = fract((p3_100.x + p3_100.y) * p3_100.z);

  var p3_010 = fract(i010 * k); p3_010 += dot(p3_010, p3_010.yzx + k2);
  let h010 = fract((p3_010.x + p3_010.y) * p3_010.z);

  var p3_110 = fract(i110 * k); p3_110 += dot(p3_110, p3_110.yzx + k2);
  let h110 = fract((p3_110.x + p3_110.y) * p3_110.z);

  var p3_001 = fract(i001 * k); p3_001 += dot(p3_001, p3_001.yzx + k2);
  let h001 = fract((p3_001.x + p3_001.y) * p3_001.z);

  var p3_101 = fract(i101 * k); p3_101 += dot(p3_101, p3_101.yzx + k2);
  let h101 = fract((p3_101.x + p3_101.y) * p3_101.z);

  var p3_011 = fract(i011 * k); p3_011 += dot(p3_011, p3_011.yzx + k2);
  let h011 = fract((p3_011.x + p3_011.y) * p3_011.z);

  var p3_111 = fract(i111 * k); p3_111 += dot(p3_111, p3_111.yzx + k2);
  let h111 = fract((p3_111.x + p3_111.y) * p3_111.z);

  // Trilinear interpolation
  let mix_x0 = mix(h000, h100, u.x);
  let mix_x1 = mix(h010, h110, u.x);
  let mix_x2 = mix(h001, h101, u.x);
  let mix_x3 = mix(h011, h111, u.x);

  let mix_y0 = mix(mix_x0, mix_x1, u.y);
  let mix_y1 = mix(mix_x2, mix_x3, u.y);

  return mix(mix_y0, mix_y1, u.z);
}

@vertex
fn vs_main(@builtin(vertex_index) v_idx: u32) -> VertexOut {
  var out: VertexOut;

  // Pre-compute time-dependent constants (same for all vertices in frame)
  let time_div_5 = uniforms.time * 0.2;
  let time_div_10 = uniforms.time * 0.1;
  let time_div_100 = uniforms.time * 0.01;
  let time_div_2 = uniforms.time * 0.5;

  let layer_count = u32(uniforms.layer_count);
  let segments_per_layer = 256u;
  let vertices_per_layer = (segments_per_layer + 1u) * 2u;

  // Pre-compute reciprocals to avoid division
  let inv_segments = 1.0 / f32(segments_per_layer);
  let inv_layer_count = 1.0 / uniforms.layer_count;
  let inv_screen_height = 2.0 / uniforms.screen_height;

  let layer_idx = v_idx / vertices_per_layer;
  let vertex_in_layer = v_idx % vertices_per_layer;
  let segment_idx = vertex_in_layer / 2u;
  let is_bottom = (vertex_in_layer % 2u) == 1u;

  // Convert to NDC coordinates
  let x_ndc = f32(segment_idx) * inv_segments * 2.0 - 1.0;
  let z_ndc = f32(layer_idx) * inv_layer_count * 2.0 - 1.0;

  var v_x = x_ndc;
  var v_z = z_ndc;

  // Initial Y from xmb_noise2
  var v_y = xmb_noise2(v_x, v_z, uniforms.time) * 0.125;

  // Prepare noise sample coordinates
  let v3_x = (v_x - time_div_5) * 0.25;
  let v3_y = v_y - time_div_100;
  let v3_z = v_z - time_div_10;

  // OPTIMIZATION: noise_1 and noise_2 used IDENTICAL inputs in original code
  // Only compute noise ONCE instead of twice
  let noise_coord = vec3(v3_x * 7.0, v3_y * 7.0, v3_z * 7.0);
  let noise_val = noise3_opt(noise_coord);

  // Apply noise to z (line 64 in original algorithm)
  v_z -= noise_val * 0.0667; // 1/15 = 0.0667

  // Apply noise + cos wave to y (line 66 in original algorithm)
  let cos_wave = cos(v_x * 2.0 - time_div_2) * 0.2;
  v_y -= (noise_val * 0.0667 + cos_wave) - 0.3;

  // Strip height for line thickness
  let stroke_width_pixels = 1.5 + (4.0 - f32(layer_idx)) * 0.3;
  let strip_height = stroke_width_pixels * inv_screen_height;
  if (is_bottom) {
    v_y -= strip_height;
  }

  out.position = vec4<f32>(v_x, v_y, 0.0, 1.0);

  // Opacity calculation with pre-computed layer factor
  let base_opacity = (150.0 - f32(layer_idx) * 30.0) * 0.00392157; // 1/255

  // Edge fade - branchless version using smoothstep
  let x_normalized = f32(segment_idx) * inv_segments;
  let edge_fade = min(x_normalized * 10.0, (1.0 - x_normalized) * 10.0);
  let final_edge_fade = clamp(edge_fade, 0.0, 1.0);

  let final_opacity = base_opacity * final_edge_fade;
  out.color = vec4<f32>(uniforms.line_color.rgb, final_opacity);
  out.layer = f32(layer_idx);

  return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
  return in.color;
}
