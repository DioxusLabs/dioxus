// Copyright © SixtyFPS GmbH <info@slint.dev>
// SPDX-License-Identifier: MIT

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) frag_position: vec2<f32>,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32
) -> VertexOutput {
    var output: VertexOutput;

    var positions = array<vec2<f32>, 3>(
        vec2<f32>(-1.0,  3.0),
        vec2<f32>(-1.0, -1.0),
        vec2<f32>( 3.0, -1.0)
    );

    let pos = positions[vertex_index];
    output.position = vec4<f32>(pos.x, -pos.y, 0.0, 1.0);
    output.frag_position = pos;
    return output;
}

struct PushConstants {
    light_color_and_time: vec4<f32>,
};

var<push_constant> pc: PushConstants;

fn sdRoundBox(p: vec3<f32>, b: vec3<f32>, r: f32) -> f32 {
    let q = abs(p) - b;
    return length(max(q, vec3<f32>(0.0))) + min(max(q.x, max(q.y, q.z)), 0.0) - r;
}

fn rotateY(r: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    let rotation_matrix = mat3x3<f32>(
        vec3<f32>( c, 0.0,  s),
        vec3<f32>(0.0, 1.0, 0.0),
        vec3<f32>(-s, 0.0,  c)
    );
    return rotation_matrix * r;
}

fn rotateZ(r: vec3<f32>, angle: f32) -> vec3<f32> {
    let c = cos(angle);
    let s = sin(angle);
    let rotation_matrix = mat3x3<f32>(
        vec3<f32>( c, -s, 0.0),
        vec3<f32>( s,  c, 0.0),
        vec3<f32>(0.0, 0.0, 1.0)
    );
    return rotation_matrix * r;
}

// Distance from the scene
fn scene(r: vec3<f32>) -> f32 {
    let iTime = pc.light_color_and_time.w;
    let pos = rotateZ(rotateY(r + vec3<f32>(-1.0, -1.0, 4.0), iTime), iTime);
    let cube = vec3<f32>(0.5, 0.5, 0.5);
    let edge = 0.1;
    return sdRoundBox(pos, cube, edge);
}

// https://iquilezles.org/articles/normalsSDF
fn normal(pos: vec3<f32>) -> vec3<f32> {
    let e = vec2<f32>(1.0, -1.0) * 0.5773;
    let eps = 0.0005;
    return normalize(
        e.xyy * scene(pos + e.xyy * eps) +
        e.yyx * scene(pos + e.yyx * eps) +
        e.yxy * scene(pos + e.yxy * eps) +
        e.xxx * scene(pos + e.xxx * eps)
    );
}

fn render(fragCoord: vec2<f32>, light_color: vec3<f32>) -> vec4<f32> {
    var color = vec4<f32>(0.0, 0.0, 0.0, 1.0);

    var camera = vec3<f32>(1.0, 2.0, 1.0);
    var p = vec3<f32>(fragCoord.x, fragCoord.y + 1.0, -1.0);
    var dir = normalize(p - camera);

    var i = 0;
    loop {
        if (i >= 90) { break; }
        let dist = scene(p);
        if (dist < 0.0001) { break; }
        p = p + dir * dist;
        i = i + 1;
    }

    let surf_normal = normal(p);
    let light_position = vec3<f32>(2.0, 4.0, -0.5);
    var light = 7.0 + 2.0 * dot(surf_normal, light_position);
    light = light / (0.2 * pow(length(light_position - p), 3.5));

    let alpha = select(0.0, 1.0, i < 90);
    return vec4<f32>(light * light_color, alpha) * 2.0;
}

@fragment
fn fs_main(@location(0) frag_position: vec2<f32>) -> @location(0) vec4<f32> {
    let selected_light_color = pc.light_color_and_time.xyz;
    let r = vec2<f32>(0.5 * frag_position.x + 1.0, 0.5 - 0.5 * frag_position.y);
    return render(r, selected_light_color);
}