// Blit shader with neon glow effect
// Applies bloom/glow to the wave lines for a neon aesthetic

@group(0) @binding(0)
var source_texture: texture_2d<f32>;

@group(0) @binding(1)
var source_sampler: sampler;

struct VertexOut {
  @builtin(position) position: vec4<f32>,
  @location(0) uv: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) v_idx: u32) -> VertexOut {
  var out: VertexOut;

  // Fullscreen triangle
  let x = f32((v_idx << 1u) & 2u);
  let y = f32(v_idx & 2u);

  out.position = vec4<f32>(x * 2.0 - 1.0, 1.0 - y * 2.0, 0.0, 1.0);
  out.uv = vec2<f32>(x, y);

  return out;
}

@fragment
fn fs_main(in: VertexOut) -> @location(0) vec4<f32> {
  let original = textureSample(source_texture, source_sampler, in.uv);

  // Dynamic texel size based on actual texture dimensions
  let tex_size = vec2<f32>(textureDimensions(source_texture));
  let texel_size = 1.0 / tex_size;

  // Inner glow (tight, bright) - 5x5 kernel
  var inner_glow = vec4<f32>(0.0);
  for (var i = -2; i <= 2; i++) {
    for (var j = -2; j <= 2; j++) {
      let offset = vec2<f32>(f32(i), f32(j)) * texel_size * 4.0;
      let s = textureSample(source_texture, source_sampler, in.uv + offset);
      let dist = length(vec2<f32>(f32(i), f32(j)));
      let weight = exp(-dist * 0.4);
      inner_glow += s * weight;
    }
  }
  inner_glow /= 12.0;

  // Mid glow - 7x7 kernel
  var mid_glow = vec4<f32>(0.0);
  for (var i = -3; i <= 3; i++) {
    for (var j = -3; j <= 3; j++) {
      let offset = vec2<f32>(f32(i), f32(j)) * texel_size * 10.0;
      let s = textureSample(source_texture, source_sampler, in.uv + offset);
      let dist = length(vec2<f32>(f32(i), f32(j)));
      let weight = exp(-dist * 0.25);
      mid_glow += s * weight;
    }
  }
  mid_glow /= 20.0;

  // Outer glow (wide, soft) - 5x5 kernel with large offset
  var outer_glow = vec4<f32>(0.0);
  for (var i = -2; i <= 2; i++) {
    for (var j = -2; j <= 2; j++) {
      let offset = vec2<f32>(f32(i), f32(j)) * texel_size * 25.0;
      let s = textureSample(source_texture, source_sampler, in.uv + offset);
      outer_glow += s;
    }
  }
  outer_glow /= 25.0;

  // Combine all layers
  var result = original * 1.0;          // Original sharp lines
  result += inner_glow * 2.5;           // Bright inner glow
  result += mid_glow * 1.5;             // Mid-range glow
  result += outer_glow * 0.8;           // Soft outer glow

  // Boost saturation for neon pop
  let luminance = dot(result.rgb, vec3<f32>(0.299, 0.587, 0.114));
  let saturated = mix(vec3<f32>(luminance), result.rgb, 1.3);
  result = vec4<f32>(saturated, result.a);

  // Soft clamp to allow slight HDR bloom feel
  let max_val = max(max(result.r, result.g), result.b);
  if (max_val > 1.0) {
    result = result / max_val;
  }

  return result;
}
