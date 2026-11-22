struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) turn: u32,
}

@vertex
fn vs_main(@builtin(vertex_index) vertex: u32, @builtin(instance_index) turn: u32) -> Vertex {
    let corner = vec2<f32>(f32((vertex << 1u) & 2u), f32(vertex & 2u));
    return Vertex(vec4<f32>(corner * 2.0 - 1.0, 0.0, 1.0),
                  vec2<f32>(corner.x, 1.0 - corner.y), turn);
}

@group(0) @binding(0) var picture: texture_2d<f32>;

@fragment
fn fs_main(v: Vertex) -> @location(0) vec4<f32> {
    let source_size = textureDimensions(picture);
    let size = select(source_size, source_size.yx, (v.turn & 1u) != 0u);
    // Choose nearest in the turned image before reversing the turn. Reversing
    // normalized UV first chooses the other texel at a nearest-sampling tie.
    let at = min(vec2<u32>(v.uv * vec2<f32>(size)), size - vec2<u32>(1u));
    var source = at;
    switch v.turn {
        case 1u: { source = vec2<u32>(at.y, source_size.y - 1u - at.x); }
        case 2u: { source = source_size - vec2<u32>(1u) - at; }
        case 3u: { source = vec2<u32>(source_size.x - 1u - at.y, at.x); }
        default: {}
    }
    return textureLoad(picture, vec2<i32>(source), 0);
}
