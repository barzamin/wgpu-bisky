// cs: clip space

struct VertexIn {
    @location(0) pos: vec3<f32>,
}

struct VertexOut {
    @builtin(position) cs_pos: vec4<f32>,
}

@vertex
fn vert_main(i: VertexIn) -> VertexOut {
    var o: VertexOut;

    let project = mat4x4<f32>(
        1.0, 0.0, 0.0, 0.0,
        0.0, aspectRatio, 0.0, 0.0,
        0.0, 0.0, 1/(zfar-znear), (-far-near)/(2*(far-near)),
        0.0, 0.0, 0.0, 1.0,
    );
    // o.cs_pos = project * vec4<f32>(i.pos, 1.0);
    o.cs_pos = project * vec4<f32>(i.pos, 1.0);

    return o;
}

@fragment
fn frag_main(v: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(1., 0., 0., 1.);
}
