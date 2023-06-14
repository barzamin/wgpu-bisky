// cs: clip space

struct VertexIn {
    @location(0) pos: vec3<f32>,
}

struct VertexOut {
    @builtin(position) cs_pos: vec4<f32>,
}

struct Camera {
    view: mat4x4<f32>,
    project: mat4x4<f32>,
}
@group(0) @binding(0) var<uniform> camera: Camera;

@vertex
fn vert_main(i: VertexIn) -> VertexOut {
    var o: VertexOut;

    o.cs_pos = camera.project * camera.view * vec4<f32>(i.pos, 1.0);

    return o;
}

@fragment
fn frag_main(v: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(1., 0., 0., 1.);
}
