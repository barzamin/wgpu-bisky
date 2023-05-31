// cs: clip space

struct VertexOut {
    @builtin(position) cs_pos: vec4<f32>,
}

// @vertex
// fn vert_main() -> VertexOut {
//     var v: VertexOut;

//     v.cs_pos = vec4<f32>(0., 0., 0., 0.);

//     return v;
// }

// test hack
@vertex
fn vert_main(
    @builtin(vertex_index) i_vert_idx: u32,
) -> VertexOut {
    var out: VertexOut;
    let x = f32(1 - i32(i_vert_idx)) * 0.5;
    let y = f32(i32(i_vert_idx & 1u) * 2 - 1) * 0.5;
    out.cs_pos = vec4<f32>(x, y, 0.0, 1.0);
    return out;
}

@fragment
fn frag_main(v: VertexOut) -> @location(0) vec4<f32> {
    return vec4<f32>(1., 0., 0., 1.);
}
