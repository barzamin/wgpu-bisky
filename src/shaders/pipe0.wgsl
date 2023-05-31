// cs: clip space

struct VertexOut {
    @builtin(position) cs_pos: vec4<f32>,
}

@vertex
fn vert_main() -> VertexOut {
    var v: VertexOut;

    v.cs_pos = vec4<f32>(0., 0., 0., 0.);

    return v;
}

@fragment
fn frag_main(v: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(1., 0., 0., 1.);
}
