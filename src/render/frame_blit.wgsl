struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) tex_coords: vec2<f32>,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    var out: VertexOutput;

    out.tex_coords = vec2<f32>(
        f32((vertex_index << 1u) & 2u),
        f32(vertex_index & 2u),
    );
    out.position = vec4<f32>(out.tex_coords * 2.0 - 1.0, 0.0, 1.0);
    out.tex_coords.y = 1.0 - out.tex_coords.y;

    return out;
}

@group(0) @binding(0)
var frame_texture: texture_2d<f32>;
@group(0) @binding(1)
var frame_sampler: sampler;

@fragment
fn fs_main(vertex: VertexOutput) -> @location(0) vec4<f32> {
    let color = textureSample(frame_texture, frame_sampler, vertex.tex_coords);

    return vec4<f32>(color.rgb, 1.0);
}
